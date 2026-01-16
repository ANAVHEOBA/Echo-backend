pub mod config;
pub mod errors;
pub mod jobs;
pub mod middleware;
pub mod modules;
pub mod services;

use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use config::{AppConfig, DbPool, RedisPool};
use modules::auth::auth_routes;
use modules::crm_integration::routes::crm_routes;
use modules::events::event_routes;
use services::{EmailService, SmtpEmailService};

pub struct AppState {
    pub pool: DbPool,
    pub redis: RedisPool,
    pub config: Arc<AppConfig>,
    pub email_service: Arc<dyn EmailService>,
}

pub async fn create_app(pool: DbPool, redis: RedisPool, config: AppConfig) -> Router {
    let email_service = Arc::new(SmtpEmailService::new(&config));
    
    let state = Arc::new(AppState {
        pool,
        redis,
        config: Arc::new(config),
        email_service,
    });

    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .nest("/api/auth", auth_routes(state.clone()))
        .nest("/api/crm", crm_routes(state.clone()))
        .nest("/api/events", event_routes(state.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn root() -> &'static str {
    "Echo Backend - CRM Auto-Updater API"
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}