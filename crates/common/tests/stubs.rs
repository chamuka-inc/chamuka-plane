#![cfg(feature = "test-stubs")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::stubs::{MockController, MockDockerSocket, MockDrone, MockProxy};

#[test]
fn mock_controller_records_and_clears() {
    let mc = MockController::new();
    assert_eq!(mc.received_count(), 0);
    mc.clear();
    assert_eq!(mc.received_count(), 0);
}

#[test]
fn mock_drone_records_and_clears() {
    let md = MockDrone::new();
    assert_eq!(md.received_count(), 0);
    md.clear();
    assert_eq!(md.received_count(), 0);
}

#[test]
fn mock_proxy_records_and_clears() {
    let mp = MockProxy::new();
    assert_eq!(mp.received_count(), 0);
    mp.clear();
    assert_eq!(mp.received_count(), 0);
}

#[test]
fn mock_docker_lifecycle() {
    let docker = MockDockerSocket::new();
    assert_eq!(docker.container_count(), 0);

    let _id = docker.create_container(common::stubs::docker::ContainerCreateRequest {
        image: "node:24".to_string(),
        name: "test-container".to_string(),
        env: Default::default(),
    });
    assert_eq!(docker.container_count(), 1);
    assert_eq!(
        docker.container_state(0),
        Some(common::stubs::docker::ContainerState::Created)
    );

    assert!(docker.start_container(0));
    assert_eq!(
        docker.container_state(0),
        Some(common::stubs::docker::ContainerState::Running)
    );

    assert!(docker.stop_container(0));
    assert_eq!(
        docker.container_state(0),
        Some(common::stubs::docker::ContainerState::Stopped)
    );

    assert_eq!(docker.container_image(0), Some("node:24".to_string()));
    assert!(!docker.start_container(99));

    docker.clear();
    assert_eq!(docker.container_count(), 0);
}

#[test]
fn mock_docker_default() {
    let docker = MockDockerSocket::default();
    assert_eq!(docker.container_count(), 0);
}
