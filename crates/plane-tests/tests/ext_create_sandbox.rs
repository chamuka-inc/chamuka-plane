use common::test_env::TestEnvironment;
use plane::{controller::ControllerServer, database::PlaneDatabase};
use plane_common::{
    names::{ControllerName, Name},
    types::{ClusterName, DronePoolName},
};
use plane_test_macro::plane_test;
use serde_json::json;
use tokio::net::TcpListener;
use url::Url;

mod common;

const TEST_CLUSTER: &str = "plane.test";

fn api_url(base: &Url, path: &str) -> String {
    format!("{}pub/v1{path}", base.as_str())
}

async fn controller_with_cluster(db: PlaneDatabase) -> ControllerServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url: Url = format!("http://{}", listener.local_addr().unwrap())
        .parse()
        .unwrap();
    let cluster: ClusterName = TEST_CLUSTER.parse().unwrap();

    ControllerServer::run_with_listener(
        db,
        listener,
        ControllerName::new_random(),
        url,
        Some(cluster),
        None,
        None,
        None,
        2,
        None,
    )
    .await
    .expect("Unable to construct controller.")
}

#[plane_test]
async fn create_sandbox_happy_path(env: TestEnvironment) {
    let db = env.db().await;
    let controller = controller_with_cluster(db).await;
    let _drone = env
        .drone_in_pool(&controller, &DronePoolName::from("untrusted"))
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let url = api_url(&controller.url(), "/sandboxes");
    let client = reqwest::Client::new();

    let response = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&json!({
            "runtime": "node22",
            "timeout_ms": 10000
        }))
        .send()
        .await
        .unwrap();

    let status = response.status();
    let body_text = response.text().await.unwrap();
    assert_eq!(status, 201, "Expected 201 Created, body: {body_text}");
    let body: serde_json::Value = serde_json::from_str(&body_text).unwrap();

    assert!(body["sandbox"]["id"].as_str().unwrap().starts_with("sb-"));
    assert!(body["sandbox"]["backend_id"].as_str().is_some());
    assert_eq!(body["sandbox"]["runtime"], "node22");
    assert_eq!(body["sandbox"]["trust"], "untrusted");
    assert_eq!(body["sandbox"]["image"], "sandbox-runtime:node22-latest");
    assert_eq!(body["sandbox"]["resources"]["vcpus"], 2);
    assert_eq!(body["sandbox"]["resources"]["memory_mb"], 4096);

    assert!(body["connection"]["token"].as_str().is_some());
    assert!(body["connection"]["url"].as_str().is_some());
    assert!(body["connection"]["status_url"].as_str().is_some());
}

#[plane_test]
async fn create_sandbox_custom_runtime_requires_image(env: TestEnvironment) {
    let db = env.db().await;
    let controller = controller_with_cluster(db).await;

    let url = api_url(&controller.url(), "/sandboxes");
    let client = reqwest::Client::new();

    let response = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&json!({
            "runtime": "custom"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], "INVALID_REQUEST");
}

#[plane_test]
async fn create_sandbox_with_key_reuses_backend(env: TestEnvironment) {
    let db = env.db().await;
    let controller = controller_with_cluster(db).await;
    let _drone = env
        .drone_in_pool(&controller, &DronePoolName::from("untrusted"))
        .await;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let url = api_url(&controller.url(), "/sandboxes");
    let client = reqwest::Client::new();
    let body = json!({
        "runtime": "node22",
        "timeout_ms": 30000,
        "key": { "name": "reuse-me" }
    });

    let r1 = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&body)
        .send()
        .await
        .unwrap();
    let s1 = r1.status();
    let b1: serde_json::Value = r1.json().await.unwrap();
    assert_eq!(s1, 201, "first create should be 201, got body: {b1}");
    let sid1 = b1["sandbox"]["id"].as_str().unwrap().to_string();
    let bid1 = b1["sandbox"]["backend_id"].as_str().unwrap().to_string();

    let r2 = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&body)
        .send()
        .await
        .unwrap();
    let s2 = r2.status();
    let b2: serde_json::Value = r2.json().await.unwrap();
    assert_eq!(s2, 200, "reuse should be 200 OK, got body: {b2}");
    assert_eq!(b2["sandbox"]["id"], sid1, "same sandbox id on reuse");
    assert_eq!(
        b2["sandbox"]["backend_id"], bid1,
        "same backend_id on reuse"
    );
    assert!(b2["connection"]["token"].as_str().is_some());
}

#[plane_test]
async fn create_sandbox_key_is_tenant_scoped(env: TestEnvironment) {
    let db = env.db().await;
    let controller = controller_with_cluster(db).await;
    let _drone = env
        .drone_in_pool(&controller, &DronePoolName::from("untrusted"))
        .await;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let url = api_url(&controller.url(), "/sandboxes");
    let client = reqwest::Client::new();
    let body = json!({
        "runtime": "node22",
        "timeout_ms": 30000,
        "key": { "name": "shared-key" }
    });

    let r_a = client
        .post(&url)
        .header("x-auth-tenant", "tenant-a")
        .json(&body)
        .send()
        .await
        .unwrap();
    let body_a: serde_json::Value = r_a.json().await.unwrap();
    let bid_a = body_a["sandbox"]["backend_id"]
        .as_str()
        .unwrap()
        .to_string();

    let r_b = client
        .post(&url)
        .header("x-auth-tenant", "tenant-b")
        .json(&body)
        .send()
        .await
        .unwrap();
    let body_b: serde_json::Value = r_b.json().await.unwrap();
    let bid_b = body_b["sandbox"]["backend_id"]
        .as_str()
        .unwrap()
        .to_string();

    assert_ne!(
        bid_a, bid_b,
        "same key.name across tenants must not reuse the same backend"
    );
}

#[plane_test]
async fn create_sandbox_reuse_extends_expires_at(env: TestEnvironment) {
    let db = env.db().await;
    let controller = controller_with_cluster(db).await;
    let _drone = env
        .drone_in_pool(&controller, &DronePoolName::from("untrusted"))
        .await;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let url = api_url(&controller.url(), "/sandboxes");
    let client = reqwest::Client::new();

    let body_short = json!({
        "runtime": "node22",
        "timeout_ms": 30000,
        "key": { "name": "extend-on-reuse" }
    });
    let body_long = json!({
        "runtime": "node22",
        "timeout_ms": 300000,
        "key": { "name": "extend-on-reuse" }
    });

    let r1 = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&body_short)
        .send()
        .await
        .unwrap();
    let b1: serde_json::Value = r1.json().await.unwrap();
    let exp1 = b1["sandbox"]["expires_at"].as_str().unwrap().to_string();

    let r2 = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&body_long)
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 200);
    let b2: serde_json::Value = r2.json().await.unwrap();
    let exp2 = b2["sandbox"]["expires_at"].as_str().unwrap().to_string();

    let exp1_t = chrono::DateTime::parse_from_rfc3339(&exp1).unwrap();
    let exp2_t = chrono::DateTime::parse_from_rfc3339(&exp2).unwrap();
    assert!(
        exp2_t > exp1_t,
        "reuse with longer timeout_ms must extend expires_at forward; exp1={exp1} exp2={exp2}"
    );

    let status = b2["sandbox"]["status"].as_str().unwrap();
    assert_ne!(
        status, "terminated",
        "reuse must not report a live backend as terminated; got status={status}"
    );
}

#[plane_test]
async fn create_sandbox_reuse_rejects_ports_mismatch(env: TestEnvironment) {
    let db = env.db().await;
    let controller = controller_with_cluster(db).await;
    let _drone = env
        .drone_in_pool(&controller, &DronePoolName::from("untrusted"))
        .await;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let url = api_url(&controller.url(), "/sandboxes");
    let client = reqwest::Client::new();
    let original = json!({
        "runtime": "node22",
        "timeout_ms": 30000,
        "ports": [8080],
        "key": { "name": "ports-shape" }
    });
    let mismatched_ports = json!({
        "runtime": "node22",
        "timeout_ms": 30000,
        "ports": [8080, 9090],
        "key": { "name": "ports-shape" }
    });

    let r1 = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&original)
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 201);

    let r2 = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&mismatched_ports)
        .send()
        .await
        .unwrap();
    assert_eq!(
        r2.status(),
        409,
        "reuse with a different ports list than the existing backend must 409"
    );
}

#[plane_test]
async fn create_sandbox_rejects_key_namespace_with_newline(env: TestEnvironment) {
    let db = env.db().await;
    let controller = controller_with_cluster(db).await;

    let url = api_url(&controller.url(), "/sandboxes");
    let client = reqwest::Client::new();
    let r = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&json!({
            "runtime": "node22",
            "key": { "name": "ok", "namespace": "x\n[FAKE LOG] hi" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["error"]["code"], "INVALID_REQUEST");
}

#[plane_test]
async fn create_sandbox_reuse_rejects_shape_mismatch(env: TestEnvironment) {
    let db = env.db().await;
    let controller = controller_with_cluster(db).await;
    let _drone = env
        .drone_in_pool(&controller, &DronePoolName::from("untrusted"))
        .await;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let url = api_url(&controller.url(), "/sandboxes");
    let client = reqwest::Client::new();

    let original = json!({
        "runtime": "node22",
        "timeout_ms": 30000,
        "resources": { "vcpus": 2 },
        "key": { "name": "shape-key" }
    });
    let mismatched_runtime = json!({
        "runtime": "python313",
        "timeout_ms": 30000,
        "resources": { "vcpus": 2 },
        "key": { "name": "shape-key" }
    });

    let r1 = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&original)
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 201);

    let r2 = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&mismatched_runtime)
        .send()
        .await
        .unwrap();
    assert_eq!(
        r2.status(),
        409,
        "reuse with a different runtime than the existing backend must 409"
    );
    let body: serde_json::Value = r2.json().await.unwrap();
    assert_eq!(body["error"]["code"], "KEY_CONFLICT");
}

#[plane_test]
async fn create_sandbox_rejects_tenant_with_slash(env: TestEnvironment) {
    let db = env.db().await;
    let controller = controller_with_cluster(db).await;

    let url = api_url(&controller.url(), "/sandboxes");
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("x-auth-tenant", "evil/tenant")
        .json(&json!({ "runtime": "node22" }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], "UNAUTHENTICATED");
}

#[plane_test]
async fn create_sandbox_no_drone_returns_503(env: TestEnvironment) {
    let db = env.db().await;
    let controller = controller_with_cluster(db).await;

    let url = api_url(&controller.url(), "/sandboxes");
    let client = reqwest::Client::new();

    let response = client
        .post(&url)
        .header("x-auth-tenant", "test-tenant")
        .json(&json!({
            "runtime": "node22"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 503);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], "NO_DRONE_AVAILABLE");
}
