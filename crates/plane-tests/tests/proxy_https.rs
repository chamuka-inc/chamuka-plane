use common::{
    simple_axum_server::{RequestInfo, SimpleAxumServer},
    test_env::TestEnvironment,
};
use plane::proxy::proxy_server::ProxyState;
use plane_common::{
    log_types::BackendAddr,
    names::{BackendName, Name},
    protocol::{RouteInfoRequest, RouteInfoResponse, SubdomainPortRouteUpdate},
    types::{BearerToken, ClusterName, SecretToken},
};
use plane_dynamic_proxy::server::{HttpsConfig, SimpleHttpServer};
use plane_test_macro::plane_test;
use rcgen::generate_simple_self_signed;
use reqwest::StatusCode;
use rustls::{
    crypto::ring::sign::any_supported_type,
    pki_types::PrivateKeyDer,
    server::{ClientHello, ResolvesServerCert},
    sign::CertifiedKey,
};
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc};

mod common;

const CERT_DOMAIN: &str = "plane.test";

#[derive(Debug)]
struct TestCertResolver {
    certified_key: Arc<CertifiedKey>,
}

impl TestCertResolver {
    fn new() -> (Self, reqwest::Certificate) {
        let rcgen::CertifiedKey { cert, key_pair } =
            generate_simple_self_signed(vec![CERT_DOMAIN.to_string(), format!("*.{CERT_DOMAIN}")])
                .expect("Failed to generate self-signed cert");

        let key = PrivateKeyDer::try_from(key_pair.serialized_der())
            .expect("Could not convert key pair to DER");
        let key = any_supported_type(&key).expect("Could not convert key to supported type");
        let cert_der = cert.der().clone();
        let reqwest_cert = reqwest::Certificate::from_der(&cert_der).unwrap();
        let certified_key = Arc::new(CertifiedKey::new(vec![cert_der], key));

        (Self { certified_key }, reqwest_cert)
    }
}

impl ResolvesServerCert for TestCertResolver {
    fn resolve(&self, _client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        Some(self.certified_key.clone())
    }
}

struct MockProxyHttps {
    route_info_request_receiver: mpsc::Receiver<RouteInfoRequest>,
    proxy_state: ProxyState,
    addr: SocketAddr,
    _server: SimpleHttpServer,
    cert: reqwest::Certificate,
}

impl MockProxyHttps {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind listener");
        let addr = listener.local_addr().expect("Failed to get local address");

        let cluster: ClusterName = format!("{CERT_DOMAIN}:{}", addr.port()).parse().unwrap();
        let proxy_state = ProxyState::new(None, cluster);
        let (route_info_request_sender, route_info_request_receiver) = mpsc::channel(1);

        proxy_state.inner.route_map.set_sender(move |m| {
            route_info_request_sender
                .try_send(m)
                .expect("Failed to send route info request");
        });

        let (resolver, cert) = TestCertResolver::new();

        let server = SimpleHttpServer::new(
            proxy_state.clone(),
            listener,
            HttpsConfig::from_resolver(resolver),
        )
        .expect("Failed to create HTTPS server");

        Self {
            route_info_request_receiver,
            proxy_state,
            addr,
            _server: server,
            cert,
        }
    }

    fn port(&self) -> u16 {
        self.addr.port()
    }

    async fn recv_route_info_request(&mut self) -> RouteInfoRequest {
        self.route_info_request_receiver
            .recv()
            .await
            .expect("Failed to receive route info request")
    }

    async fn send_route_info_response(&mut self, response: RouteInfoResponse) {
        self.proxy_state.inner.route_map.receive(response);
    }
}

#[plane_test]
async fn proxy_https_bearer_token_routing(env: TestEnvironment) {
    let server = SimpleAxumServer::new().await;
    let mut proxy = MockProxyHttps::new().await;
    let port = proxy.port();

    let client = reqwest::Client::builder()
        .https_only(true)
        .add_root_certificate(proxy.cert.clone())
        .resolve(CERT_DOMAIN, proxy.addr)
        .build()
        .unwrap();

    let url = format!("https://{CERT_DOMAIN}:{port}/mytoken123/some/path");
    let handle = tokio::spawn(client.get(url).send());

    let request = proxy.recv_route_info_request().await;
    assert_eq!(request.token, BearerToken::from("mytoken123".to_string()));

    let cluster = ClusterName::from_str(&format!("{CERT_DOMAIN}:{port}")).unwrap();
    proxy
        .send_route_info_response(RouteInfoResponse {
            token: BearerToken::from("mytoken123".to_string()),
            route_info: Some(plane_common::protocol::RouteInfo {
                backend_id: BackendName::new_random(),
                address: BackendAddr(server.addr()),
                secret_token: SecretToken::from("secret".to_string()),
                cluster,
                user: None,
                user_data: None,
                subdomain: None,
            }),
        })
        .await;

    let response = handle.await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let request_info: RequestInfo = response.json().await.unwrap();
    assert_eq!(request_info.path, "/some/path");
    assert_eq!(
        request_info.headers.get("x-forwarded-proto").unwrap(),
        "https"
    );
}

#[plane_test]
async fn proxy_https_subdomain_routing(env: TestEnvironment) {
    let server = SimpleAxumServer::new().await;
    let mut proxy = MockProxyHttps::new().await;
    let port = proxy.port();

    proxy
        .proxy_state
        .inner
        .subdomain_port_map
        .update(SubdomainPortRouteUpdate {
            sandbox_id_prefix: "sb-test".to_string(),
            port: 3000,
            address: BackendAddr(server.addr()),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        });

    let client = reqwest::Client::builder()
        .https_only(true)
        .add_root_certificate(proxy.cert.clone())
        .resolve(&format!("sb-test-3000.{CERT_DOMAIN}"), proxy.addr)
        .build()
        .unwrap();

    let url = format!("https://sb-test-3000.{CERT_DOMAIN}:{port}/hello");
    let response = client.get(url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let request_info: RequestInfo = response.json().await.unwrap();
    assert_eq!(request_info.path, "/hello");
}
