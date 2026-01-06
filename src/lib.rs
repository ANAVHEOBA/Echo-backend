pub mod config;
pub mod errors;
pub mod jobs;
pub mod middleware;
pub mod modules;
pub mod queue;
pub mod services;

use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use config::{AppConfig, DbPool};
use modules::auth::auth_routes;

pub struct AppState {
    pub pool: DbPool,
    pub config: Arc<AppConfig>,
}

pub async fn create_app(pool: DbPool, config: AppConfig) -> Router {
    let state = Arc::new(AppState {
        pool,
        config: Arc::new(config),
    });

    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .nest("/api/auth", auth_routes())
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
