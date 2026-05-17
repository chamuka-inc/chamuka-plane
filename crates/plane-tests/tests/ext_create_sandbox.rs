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
    assert_eq!(body["sandbox"]["image"], "node:22-slim");
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
