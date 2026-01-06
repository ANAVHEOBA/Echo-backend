use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    let app = echo_backend::create_app(pool, config).await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
