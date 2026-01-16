use axum::{middleware, routing::{get, post, put, delete}, Router};
use std::sync::Arc;

use crate::AppState;
use crate::middleware::require_auth;
use super::controllers;

pub fn crm_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    // Public routes - no authentication required
    let public_routes = Router::new()
        .route("/webhooks", post(controllers::handle_webhook));

    // Protected routes - require valid JWT
    let protected_routes = Router::new()
        .route("/contacts", get(controllers::list_contacts).post(controllers::create_contact))
        .route("/contacts/{id}", get(controllers::get_contact))
        .route("/contacts/{id}", put(controllers::update_contact).patch(controllers::update_contact))
        .route("/contacts/{id}", delete(controllers::delete_contact))
        .route("/leads", get(controllers::list_leads).post(controllers::create_lead))
        .route("/leads/{id}", get(controllers::get_lead))
        .route("/leads/{id}", put(controllers::update_lead).patch(controllers::update_lead))
        .route("/leads/{id}", delete(controllers::delete_lead))
        .route("/leads/{id}/convert", post(controllers::convert_lead_to_opportunity))
        .route("/opportunities", get(controllers::list_opportunities).post(controllers::create_opportunity))
        .route("/opportunities/{id}", get(controllers::get_opportunity))
        .route("/opportunities/{id}", put(controllers::update_opportunity).patch(controllers::update_opportunity))
        .route("/opportunities/{id}/stage", put(controllers::update_opportunity_stage).patch(controllers::update_opportunity_stage))
        .route("/opportunities/{id}", delete(controllers::delete_opportunity))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
}