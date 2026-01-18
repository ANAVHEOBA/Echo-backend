use axum::{
    routing::{get, post, delete},
    Router,
    middleware,
    http::StatusCode,
};
use std::sync::Arc;

use crate::AppState;
use super::controllers;
use crate::middleware::require_auth;

pub fn event_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Public Webhook Endpoints (No Auth Required)
    let public_routes = Router::new()
        .route("/webhooks/slack", post(controllers::slack_webhook))
        .route("/webhooks/gmail", post(controllers::gmail_webhook))
        .route("/webhooks/zoom", post(controllers::zoom_webhook))
        .route("/webhooks/generic", post(controllers::generic_webhook));

    // Protected Management Endpoints (Require Auth)
    let protected_routes = Router::new()
        .route("/", get(controllers::list_events))
        .route("/stats", get(controllers::get_event_stats)) // Specific path before dynamic :id
        .route("/dead-letter-queue", get(|| async { StatusCode::NOT_FOUND })) // Placeholder for DLQ
        .route("/subscriptions", post(controllers::create_subscription).get(controllers::list_subscriptions))
        .route("/subscriptions/{id}", delete(controllers::delete_subscription))
        .route("/replay/{id}", post(controllers::replay_event))
        .route("/{id}", get(controllers::get_event)) // Dynamic path last
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
}