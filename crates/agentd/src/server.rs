use axum::routing::{get, post, put};
use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::handlers;

pub fn router() -> Router {
    Router::new()
        .route("/health", get(handlers::health::get))
        .route("/info", get(handlers::info::get))
        .route("/v1/exec", post(handlers::exec::post))
        .route("/v1/fs/read", get(handlers::fs::read))
        .route("/v1/fs/write", put(handlers::fs::write))
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
