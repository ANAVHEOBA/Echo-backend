use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::ApiError;
use super::model::User;
use super::services::{hash_password, verify_password, hash_token, create_access_token, create_refresh_token, validate_refresh_token};

pub struct UserCrud<'a> {
    pool: &'a PgPool,
    config: &'a Arc<AppConfig>,
}

pub struct LoginResult {
    pub access_token: String,
    pub refresh_token: String,
}

impl<'a> UserCrud<'a> {
    pub fn new(pool: &'a PgPool, config: &'a Arc<AppConfig>) -> Self {
        Self { pool, config }
    }

    pub async fn create_user(&self, email: &str, password: &str, first_name: Option<&str>, last_name: Option<&str>) -> Result<User, ApiError> {
        let password_hash = hash_password(password)?;

        sqlx::query_as::<_, User>(
            r#"INSERT INTO users (email, password_hash, first_name, last_name, role)
               VALUES ($1, $2, $3, $4, 'member') RETURNING *"#,
        )
        .bind(email.to_lowercase().trim())
        .bind(&password_hash)
        .bind(first_name)
        .bind(last_name)
        .fetch_one(self.pool)
        .await
        .map_err(ApiError::from)
    }

    pub async fn find_by_id(&self, id: &Uuid) -> Result<Option<User>, ApiError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(ApiError::from)
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, ApiError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE LOWER(email) = LOWER($1)")
            .bind(email.trim())
            .fetch_optional(self.pool)
            .await
            .map_err(ApiError::from)
    }

    pub async fn email_exists(&self, email: &str) -> Result<bool, ApiError> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE LOWER(email) = LOWER($1)")
            .bind(email.trim())
            .fetch_one(self.pool)
            .await
            .map_err(ApiError::from)?;
        Ok(result.0 > 0)
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<LoginResult, ApiError> {
        let user = self.find_by_email(email).await?.ok_or(ApiError::InvalidCredentials)?;

        if user.is_deleted() {
            return Err(ApiError::AccountDeleted);
        }
        if user.is_locked() {
            return Err(ApiError::AccountLocked(user.locked_until.unwrap().to_rfc3339()));
        }

        if !verify_password(password, &user.password_hash)? {
            self.increment_failed_login(user.id).await?;
            return Err(ApiError::InvalidCredentials);
        }

        self.reset_failed_login(user.id).await?;

        let access_token = create_access_token(&user, &self.config)?;
        let refresh_token = create_refresh_token(&user, &self.config)?;

        self.store_refresh_token(user.id, &refresh_token).await?;

        Ok(LoginResult { access_token, refresh_token })
    }

    pub async fn refresh_tokens(&self, token: &str) -> Result<LoginResult, ApiError> {
        let claims = validate_refresh_token(token, &self.config)?;

        let token_hash = hash_token(token);
        let stored: Option<(Uuid, bool, chrono::DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, revoked, expires_at FROM refresh_tokens WHERE token_hash = $1"
        )
        .bind(&token_hash)
        .fetch_optional(self.pool)
        .await
        .map_err(ApiError::from)?;

        let (id, revoked, expires_at) = stored.ok_or(ApiError::InvalidToken)?;
        if revoked { return Err(ApiError::TokenRevoked); }
        if expires_at < Utc::now() { return Err(ApiError::TokenExpired); }

        // Revoke old
        sqlx::query("UPDATE refresh_tokens SET revoked = TRUE, revoked_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(ApiError::from)?;

        let user = self.find_by_id(&claims.sub).await?.ok_or(ApiError::UserNotFound)?;

        let access_token = create_access_token(&user, &self.config)?;
        let new_refresh = create_refresh_token(&user, &self.config)?;

        self.store_refresh_token(user.id, &new_refresh).await?;

        Ok(LoginResult { access_token, refresh_token: new_refresh })
    }

    pub async fn logout(&self, token: &str) -> Result<(), ApiError> {
        let token_hash = hash_token(token);
        sqlx::query("UPDATE refresh_tokens SET revoked = TRUE, revoked_at = NOW() WHERE token_hash = $1")
            .bind(&token_hash)
            .execute(self.pool)
            .await
            .map_err(ApiError::from)?;
        Ok(())
    }

    async fn store_refresh_token(&self, user_id: Uuid, token: &str) -> Result<(), ApiError> {
        let token_hash = hash_token(token);
        let expires_at = Utc::now() + Duration::days(self.config.refresh_token_expiry_days);

        sqlx::query("INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(&token_hash)
            .bind(expires_at)
            .execute(self.pool)
            .await
            .map_err(ApiError::from)?;
        Ok(())
    }

    async fn increment_failed_login(&self, user_id: Uuid) -> Result<(), ApiError> {
        let attempts: i32 = sqlx::query_scalar(
            "UPDATE users SET failed_login_attempts = failed_login_attempts + 1 WHERE id = $1 RETURNING failed_login_attempts"
        )
        .bind(user_id)
        .fetch_one(self.pool)
        .await
        .map_err(ApiError::from)?;

        if attempts >= 5 {
            sqlx::query("UPDATE users SET locked_until = $1 WHERE id = $2")
                .bind(Utc::now() + Duration::minutes(30))
                .bind(user_id)
                .execute(self.pool)
                .await
                .map_err(ApiError::from)?;
        }
        Ok(())
    }

    async fn reset_failed_login(&self, user_id: Uuid) -> Result<(), ApiError> {
        sqlx::query("UPDATE users SET failed_login_attempts = 0, locked_until = NULL WHERE id = $1")
            .bind(user_id)
            .execute(self.pool)
            .await
            .map_err(ApiError::from)?;
        Ok(())
    }
}
