pub mod api_v1;
pub mod dispatch;
pub mod error;
pub mod types;

#[cfg(test)]
mod tests;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::controller::core::Controller;

pub fn ext_routes() -> Router<Controller> {
    Router::new()
        .route("/sandboxes", post(api_v1::create_sandbox))
        .route("/sandboxes", get(api_v1::list_sandboxes))
        .route("/sandboxes/:id", get(api_v1::get_sandbox))
        .route("/sandboxes/:id", delete(api_v1::delete_sandbox))
        .route("/sandboxes/:id/extend", post(api_v1::extend_sandbox))
        .route("/sandboxes/:id/status", get(api_v1::sandbox_status))
        .route("/sandboxes/:id/snapshot", post(api_v1::snapshot_sandbox))
        .route(
            "/sandboxes/:id/network-policy",
            post(api_v1::update_network_policy),
        )
        .route(
            "/sandboxes/:id/credentials",
            post(api_v1::create_credential_binding),
        )
        .route(
            "/sandboxes/:id/credentials/:cred_id",
            delete(api_v1::delete_credential_binding),
        )
        .route("/snapshots/:id", get(api_v1::get_snapshot))
        .route("/snapshots/:id", delete(api_v1::delete_snapshot))
        .route("/secrets", post(api_v1::create_secret))
        .route("/secrets/:id", delete(api_v1::delete_secret))
}
