use common::test_env::TestEnvironment;
use plane_common::types::DronePoolName;
use plane_test_macro::plane_test;
use serde_json::json;

mod common;

#[plane_test]
async fn create_sandbox_happy_path(env: TestEnvironment) {
    let controller = env.controller().await;
    let _drone = env
        .drone_in_pool(&controller, &DronePoolName::from("untrusted"))
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let url = format!("{}/pub/v1/sandboxes", controller.url());
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

    assert_eq!(response.status(), 201, "Expected 201 Created");

    let body: serde_json::Value = response.json().await.unwrap();

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
    let controller = env.controller().await;

    let url = format!("{}/pub/v1/sandboxes", controller.url());
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
    let controller = env.controller().await;

    let url = format!("{}/pub/v1/sandboxes", controller.url());
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
