use common::{
    proxy_mock::MockProxy,
    simple_axum_server::{RequestInfo, SimpleAxumServer},
    test_env::TestEnvironment,
};
use plane_common::{
    log_types::BackendAddr,
    names::{BackendName, Name},
    protocol::{RouteInfoResponse, SubdomainPortRouteUpdate},
    types::{BearerToken, SecretToken},
};
use plane_test_macro::plane_test;
use rcgen::generate_simple_self_signed;
use reqwest::StatusCode;
use rustls::{
    crypto::ring::sign::any_supported_type,
    pki_types::PrivateKeyDer,
    server::{ClientHello, ResolvesServerCert},
    sign::CertifiedKey,
};
use std::sync::Arc;

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

#[plane_test]
async fn proxy_https_bearer_token_routing(env: TestEnvironment) {
    let server = SimpleAxumServer::new().await;
    let (resolver, cert) = TestCertResolver::new();
    let mut proxy = MockProxy::new_with_https(CERT_DOMAIN, resolver, cert.clone()).await;
    let port = proxy.port();

    let client = reqwest::Client::builder()
        .https_only(true)
        .add_root_certificate(cert)
        .resolve(CERT_DOMAIN, proxy.addr())
        .build()
        .unwrap();

    let token = "mytoken123";
    let url = format!("https://{CERT_DOMAIN}:{port}/{token}/some/path");
    let handle = tokio::spawn(client.get(url).send());

    let request = proxy.recv_route_info_request().await;
    assert_eq!(request.token, BearerToken::from(token.to_string()));

    let cluster = proxy.cluster();
    proxy
        .send_route_info_response(RouteInfoResponse {
            token: BearerToken::from(token.to_string()),
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
    let (resolver, cert) = TestCertResolver::new();
    let mut proxy = MockProxy::new_with_https(CERT_DOMAIN, resolver, cert.clone()).await;
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

    let subdomain_host = format!("sb-test-3000.{CERT_DOMAIN}");
    let client = reqwest::Client::builder()
        .https_only(true)
        .add_root_certificate(cert)
        .resolve(&subdomain_host, proxy.addr())
        .build()
        .unwrap();

    let url = format!("https://{subdomain_host}:{port}/hello");
    let response = client.get(url).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let request_info: RequestInfo = response.json().await.unwrap();
    assert_eq!(request_info.path, "/hello");
}
