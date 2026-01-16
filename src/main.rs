use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "echo_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = echo_backend::config::AppConfig::from_env()
        .expect("Failed to load configuration");

    let pool = echo_backend::config::create_pool(&config)
        .await
        .expect("Failed to connect to database");

    tracing::info!("Connected to PostgreSQL");

    let redis = echo_backend::config::create_redis_pool(&config)
        .await
        .expect("Failed to connect to Redis");

    tracing::info!("Connected to Redis");

    let email_service = Arc::new(echo_backend::services::SmtpEmailService::new(&config));

    // Initialize AppState for Worker
    let app_state = Arc::new(echo_backend::AppState {
        pool: pool.clone(),
        redis: redis.clone(),
        config: Arc::new(config.clone()),
        email_service,
    });

    // Start Background Worker
    let worker = echo_backend::services::worker::Worker::new(app_state);
    tokio::spawn(async move {
        worker.run().await;
    });

    let app = echo_backend::create_app(pool, redis, config).await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}