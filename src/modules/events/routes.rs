use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::AppState;
use super::controllers;

pub fn event_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        // Webhook endpoints
        .route("/webhooks/slack", post(controllers::slack_webhook))
        .route("/webhooks/gmail", post(controllers::gmail_webhook))
        .route("/webhooks/zoom", post(controllers::zoom_webhook))
        .route("/webhooks/generic", post(controllers::generic_webhook))
        
        // Event management endpoints
        .route("/events", get(controllers::list_events))
        .route("/events/stats", get(controllers::get_event_stats))
        
        .with_state(state)
}
