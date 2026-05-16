use std::net::SocketAddr;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("agentd listening on {addr}");

    axum::serve(listener, agentd::router()).await?;
    Ok(())
}
