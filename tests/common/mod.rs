use axum::Router;
use echo_backend::{create_app, config::{AppConfig, RedisPool}};
use secrecy::{SecretString, ExposeSecret};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;
use tokio::sync::OnceCell;

pub async fn create_test_config() -> AppConfig {
    // Use override to ensure .env values are used even if env vars are already set
    let _ = dotenvy::dotenv_override();
    
    // Enable TEST_MODE for auth bypass
    std::env::set_var("TEST_MODE", "1");
    
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://test:test@localhost/test_db".to_string());
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());

    AppConfig {
        database_url: SecretString::from(database_url),
        redis_url,
        jwt_secret: SecretString::from("test_jwt_secret_key_that_is_at_least_32_characters_long"),
        jwt_refresh_secret: SecretString::from("test_refresh_secret_key_that_is_at_least_32_chars"),
        access_token_expiry_secs: 900,
        refresh_token_expiry_days: 7,
        encryption_key: SecretString::from("test_encryption_key_32_chars_ok"),
        host: "127.0.0.1".to_string(),
        port: 3000,
        smtp_host: "smtp.gmail.com".to_string(),
        smtp_port: 587,
        smtp_username: "test@example.com".to_string(),
        smtp_password: SecretString::from("test_password"),
        email_from: "test@example.com".to_string(),
        google_client_id: std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID must be set in .env"),
        google_client_secret: SecretString::from(
            std::env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET must be set in .env")
        ),
        zoom_client_id: "test_zoom_client_id".to_string(),
        zoom_client_secret: SecretString::from("test_zoom_client_secret"),
        zoom_webhook_secret: SecretString::from("test_zoom_webhook_secret"),
        generic_webhook_secret: SecretString::from("test_signature"),
        slack_signing_secret: SecretString::from("test_slack_secret"),
    }
}

static MIGRATION_LOCK: OnceCell<()> = OnceCell::const_new();

pub async fn setup_test_context() -> (Router, Arc<AppConfig>, PgPool, RedisPool) {
    let config = create_test_config().await;
    
    // Connect to database
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(config.database_url.expose_secret())
        .await
        .expect("Failed to connect to test database");
    
    // Run migrations ONLY ONCE
    MIGRATION_LOCK.get_or_init(|| async {
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        // Insert test user for test_token (Uuid::nil())
        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, role, email_verified)
            VALUES ($1, 'test@example.com', 'hash', 'admin', true)
            ON CONFLICT (id) DO NOTHING
            "#
        )
        .bind(uuid::Uuid::nil())
        .execute(&pool)
        .await
        .expect("Failed to seed test user");
    }).await;

    let redis = echo_backend::config::create_redis_pool(&config).await
        .expect("Failed to create redis pool");

    let app = create_app(pool.clone(), redis.clone(), config.clone()).await;
    
    (app, Arc::new(config), pool, redis)
}

pub async fn create_test_app() -> Router {
    let (app, _, _, _) = setup_test_context().await;
    app
}