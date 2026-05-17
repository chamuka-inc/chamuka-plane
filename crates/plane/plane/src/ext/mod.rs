pub mod api_v1;
pub mod error;
pub mod types;

#[cfg(test)]
mod tests;

use axum::{routing::post, Router};

use crate::controller::core::Controller;

pub fn ext_routes() -> Router<Controller> {
    Router::new().route("/sandboxes", post(api_v1::create_sandbox))
}
