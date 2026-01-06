use secrecy::{ExposeSecret, SecretString};
use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: SecretString,
    pub redis_url: String,
    pub jwt_secret: SecretString,
    pub jwt_refresh_secret: SecretString,
    pub access_token_expiry_secs: i64,
    pub refresh_token_expiry_days: i64,
    pub encryption_key: SecretString,
    pub host: String,
    pub port: u16,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        Ok(Self {
            database_url: SecretString::from(
                env::var("DATABASE_URL")
                    .map_err(|_| ConfigError::Missing("DATABASE_URL"))?,
            ),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            jwt_secret: SecretString::from(
                env::var("JWT_SECRET")
                    .map_err(|_| ConfigError::Missing("JWT_SECRET"))?,
            ),
            jwt_refresh_secret: SecretString::from(
                env::var("JWT_REFRESH_SECRET")
                    .map_err(|_| ConfigError::Missing("JWT_REFRESH_SECRET"))?,
            ),
            access_token_expiry_secs: env::var("ACCESS_TOKEN_EXPIRY_SECS")
                .unwrap_or_else(|_| "900".to_string())
                .parse()
                .map_err(|_| ConfigError::Invalid("ACCESS_TOKEN_EXPIRY_SECS"))?,
            refresh_token_expiry_days: env::var("REFRESH_TOKEN_EXPIRY_DAYS")
                .unwrap_or_else(|_| "7".to_string())
                .parse()
                .map_err(|_| ConfigError::Invalid("REFRESH_TOKEN_EXPIRY_DAYS"))?,
            encryption_key: SecretString::from(
                env::var("ENCRYPTION_KEY")
                    .map_err(|_| ConfigError::Missing("ENCRYPTION_KEY"))?,
            ),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .map_err(|_| ConfigError::Invalid("PORT"))?,
        })
    }

    pub fn database_url(&self) -> &str {
        self.database_url.expose_secret()
    }

    pub fn jwt_secret(&self) -> &str {
        self.jwt_secret.expose_secret()
    }

    pub fn jwt_refresh_secret(&self) -> &str {
        self.jwt_refresh_secret.expose_secret()
    }

    pub fn encryption_key(&self) -> &str {
        self.encryption_key.expose_secret()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    Missing(&'static str),
    #[error("Invalid environment variable: {0}")]
    Invalid(&'static str),
}
