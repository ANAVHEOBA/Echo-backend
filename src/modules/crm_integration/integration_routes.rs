use axum::{routing::get, Router, middleware};
use std::sync::Arc;

use crate::AppState;
use crate::middleware::require_auth;
use super::integrations_controller;

pub fn integration_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let protected_routes = Router::new()
        .route("/zoom/connect", get(integrations_controller::connect_zoom))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let public_routes = Router::new()
        .route("/zoom/callback", get(integrations_controller::zoom_callback));

    Router::new()
        .merge(protected_routes)
        .merge(public_routes)
        .with_state(state)
}
