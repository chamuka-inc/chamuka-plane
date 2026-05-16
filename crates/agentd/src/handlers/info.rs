use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct InfoResponse {
    pub version: &'static str,
    pub runtime: &'static str,
    pub arch: &'static str,
    pub os: &'static str,
}

pub async fn get() -> Json<InfoResponse> {
    Json(InfoResponse {
        version: env!("CARGO_PKG_VERSION"),
        runtime: "agentd-headless",
        arch: std::env::consts::ARCH,
        os: std::env::consts::OS,
    })
}
