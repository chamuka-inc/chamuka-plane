use std::net::SocketAddr;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let host = std::env::var("AGENTD_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("AGENTD_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("agentd listening on {addr}");

    axum::serve(listener, agentd::router()).await?;
    Ok(())
}
