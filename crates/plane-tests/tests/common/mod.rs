use self::{test_env::TestEnvironment, timeout::WithTimeout};
use futures_util::Future;
use plane_common::{names::BackendName, types::BackendStatus, PlaneClient};
use std::{panic::AssertUnwindSafe, time::Duration};
use tokio::time::timeout;

pub const TEST_IMAGE: &str = "sandbox-runtime:test-latest";

pub mod async_drop;
pub mod auth_mock;
pub mod docker;
pub mod localhost_resolver;
pub mod proxy_mock;
pub mod resources;
pub mod simple_axum_server; // TODO: copied from dynamic-proxy (until we merge them back)
pub mod test_env;
pub mod timeout;
pub mod websocket_echo_server; // TODO: copied from dynamic-proxy (until we merge them back)

pub fn run_test<F, Fut>(name: &str, time_limit: Duration, test_function: F)
where
    F: FnOnce(TestEnvironment) -> Fut,
    Fut: Future<Output = ()>,
{
    let env = TestEnvironment::new(name);

    {
        let env = env.clone();

        // Run the tests and capture a panic.
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let result = test_function(env.clone());
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build runtime")
                .block_on(async {
                    if timeout(time_limit, result).await.is_err() {
                        panic!("Test timed out after {} seconds.", time_limit.as_secs());
                    }
                });
        }));

        // Run the cleanup. We run this in a separate runtime so that it is run
        // regardless of whether the test panicked or not.
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build runtime")
            .block_on(async {
                env.cleanup().await;
            });

        // If the test panicked, resume the panic.
        if let Err(err) = result {
            tracing::error!("Test panicked: {:?}", err);
            std::panic::resume_unwind(err);
        }
    }
}

async fn wait_until_backend_status(
    client: &PlaneClient,
    backend_id: &BackendName,
    target: BackendStatus,
    poll_timeout_secs: u32,
) {
    let mut backend_status_stream = client
        .backend_status_stream(backend_id)
        .with_timeout(10)
        .await
        .unwrap()
        .unwrap();

    while let Ok(Some(message)) = backend_status_stream
        .next()
        .with_timeout(poll_timeout_secs)
        .await
    {
        tracing::info!("Got status: {:?}", message);
        if message.status == target {
            break;
        }
    }
}

#[allow(dead_code)]
pub async fn wait_until_backend_ready(client: &PlaneClient, backend_id: &BackendName) {
    wait_until_backend_status(client, backend_id, BackendStatus::Ready, 30).await;
}

#[allow(dead_code)]
pub async fn wait_until_backend_terminated(client: &PlaneClient, backend_id: &BackendName) {
    wait_until_backend_status(client, backend_id, BackendStatus::Terminated, 10).await;
}
