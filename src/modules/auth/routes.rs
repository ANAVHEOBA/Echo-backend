use axum::{middleware, routing::{get, post}, Router};
use std::sync::Arc;

use crate::AppState;
use crate::middleware::require_auth;
use super::controller;

pub fn auth_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Public routes - no authentication required
    let public_routes = Router::new()
        .route("/register", post(controller::register))
        .route("/login", post(controller::login))
        .route("/refresh", post(controller::refresh))
        .route("/logout", post(controller::logout));

    // Protected routes - require valid JWT
    let protected_routes = Router::new()
        .route("/me", get(controller::me))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
}
