use bollard::Docker;
use plane_common::{names::BackendName, types::backend_state::BackendError};
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use super::types::ContainerId;

const WAIT_TIMEOUT_MS: u64 = 100;
const REQUEST_TIMEOUT_MS: u64 = 1_000;
const STARTUP_TIMEOUT_SECS: u64 = 300; // 5 min timeout
const LIVENESS_CHECK_INTERVAL: u32 = 5; // check container every N poll iterations
const HEAD_REQUEST: &[u8] = b"HEAD / HTTP/1.1\r\nHost: localhost\r\n\r\n";

/// Waits until an HTTP request to the given socket address succeeds or times out after 5 minutes.
/// Periodically checks that the container is still running — if it exited, returns immediately
/// instead of polling TCP for the full timeout.
pub async fn wait_for_backend(
    docker: &Docker,
    backend_id: &BackendName,
    address: SocketAddr,
) -> Result<(), BackendError> {
    let container_id: ContainerId = backend_id.into();
    let container_id_str = container_id.to_string();
    let start = Instant::now();
    let timeout_duration = Duration::from_secs(STARTUP_TIMEOUT_SECS);
    let mut iteration: u32 = 0;

    loop {
        if start.elapsed() > timeout_duration {
            return Err(BackendError::StartupTimeout);
        }

        // Periodically verify the container is still alive.
        if iteration > 0 && iteration.is_multiple_of(LIVENESS_CHECK_INTERVAL) {
            match check_container_alive(docker, &container_id_str).await {
                ContainerStatus::Running => {}
                ContainerStatus::Exited(code) => {
                    return Err(BackendError::ContainerExited(code));
                }
                ContainerStatus::Gone => {
                    return Err(BackendError::ContainerExited(None));
                }
            }
        }
        iteration += 1;

        let Ok(mut conn) = TcpStream::connect(address).await else {
            tokio::time::sleep(Duration::from_millis(WAIT_TIMEOUT_MS)).await;
            continue;
        };

        if conn.write_all(HEAD_REQUEST).await.is_err() {
            tokio::time::sleep(Duration::from_millis(WAIT_TIMEOUT_MS)).await;
            continue;
        }

        let mut buffer = [0; 5]; // We expect the response to start with b"HTTP/"
        let Ok(result) = tokio::time::timeout(
            Duration::from_millis(REQUEST_TIMEOUT_MS),
            conn.read(&mut buffer),
        )
        .await
        else {
            // timed out
            tracing::warn!("Timed out reading from socket.");
            tokio::time::sleep(Duration::from_millis(WAIT_TIMEOUT_MS)).await;
            continue;
        };

        if result.is_err() {
            // error
            tokio::time::sleep(Duration::from_millis(WAIT_TIMEOUT_MS)).await;
            continue;
        }

        if &buffer == b"HTTP/" {
            // success
            break Ok(());
        } else {
            // not ready yet
            tokio::time::sleep(Duration::from_millis(WAIT_TIMEOUT_MS)).await;
            continue;
        }
    }
}

enum ContainerStatus {
    Running,
    Exited(Option<i32>),
    Gone,
}

async fn check_container_alive(docker: &Docker, container_id: &str) -> ContainerStatus {
    match docker.inspect_container(container_id, None).await {
        Ok(info) => {
            let state = info.state.as_ref();
            let running = state.and_then(|s| s.running).unwrap_or(false);
            if running {
                ContainerStatus::Running
            } else {
                let exit_code = state.and_then(|s| s.exit_code).map(|c| c as i32);
                ContainerStatus::Exited(exit_code)
            }
        }
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => ContainerStatus::Gone,
        Err(e) => {
            tracing::warn!(?e, "Failed to inspect container during liveness check");
            ContainerStatus::Running // assume alive on transient errors
        }
    }
}
