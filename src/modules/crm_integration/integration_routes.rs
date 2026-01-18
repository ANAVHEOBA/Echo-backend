use axum::{routing::get, Router};
use std::sync::Arc;

use crate::AppState;
use super::integrations_controller;

pub fn integration_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/zoom/connect", get(integrations_controller::connect_zoom))
        .route("/zoom/callback", get(integrations_controller::zoom_callback))
        .with_state(state)
}
