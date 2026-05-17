pub mod api_v1;
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
}
