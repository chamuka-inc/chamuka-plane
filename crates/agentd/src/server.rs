use axum::routing::{delete, get, post, put};
use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::handlers;
use crate::process_registry::ProcessRegistry;

pub fn router() -> Router {
    let registry = ProcessRegistry::new();

    Router::new()
        .route("/health", get(handlers::health::get))
        .route("/info", get(handlers::info::get))
        .route("/v1/exec", post(handlers::exec::post))
        .route("/v1/exec/{cmd_id}/signal", post(handlers::signal::post))
        .route("/v1/fs/read", get(handlers::fs::read))
        .route("/v1/fs/write", put(handlers::fs::write))
        .route("/v1/fs/mkdir", post(handlers::fs::mkdir))
        .route("/v1/fs/rm", delete(handlers::fs::rm))
        .route("/v1/fs/list", get(handlers::fs::list))
        .route("/v1/fs/move", post(handlers::fs::mv))
        .route("/v1/ports", get(handlers::ports::get))
        .route("/v1/processes", get(handlers::processes::get))
        .with_state(registry)
}

/// Bind to an ephemeral port and spawn the server. Returns the bound address.
/// Intended for integration tests only.
#[allow(clippy::expect_used)]
pub async fn serve_on_ephemeral() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind to ephemeral port");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, router()).await.expect("axum serve");
    });
    addr
}
