use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::ApiError;
use crate::modules::auth::model::{Claims, TokenType, User};

pub fn create_access_token(user: &User, config: &AppConfig) -> Result<String, ApiError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user.id,
        email: user.email.clone(),
        role: user.role.to_string(),
        org_id: user.organization_id,
        token_type: TokenType::Access,
        iat: now.timestamp(),
        exp: (now + Duration::seconds(config.access_token_expiry_secs)).timestamp(),
        jti: Uuid::new_v4(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret().as_bytes()),
    )
    .map_err(|e| ApiError::InternalError(format!("JWT encode failed: {}", e)))
}

pub fn create_refresh_token(user: &User, config: &AppConfig) -> Result<String, ApiError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user.id,
        email: user.email.clone(),
        role: user.role.to_string(),
        org_id: user.organization_id,
        token_type: TokenType::Refresh,
        iat: now.timestamp(),
        exp: (now + Duration::days(config.refresh_token_expiry_days)).timestamp(),
        jti: Uuid::new_v4(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_refresh_secret().as_bytes()),
    )
    .map_err(|e| ApiError::InternalError(format!("JWT encode failed: {}", e)))
}

pub fn validate_access_token(token: &str, config: &AppConfig) -> Result<Claims, ApiError> {
    let mut validation = Validation::default();
    validation.leeway = 0; // Strict expiration check, no grace period

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret().as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::TokenExpired,
        _ => ApiError::InvalidToken,
    })?;

    if data.claims.token_type != TokenType::Access {
        return Err(ApiError::InvalidToken);
    }
    Ok(data.claims)
}

pub fn validate_refresh_token(token: &str, config: &AppConfig) -> Result<Claims, ApiError> {
    let mut validation = Validation::default();
    validation.leeway = 0; // Strict expiration check, no grace period

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_refresh_secret().as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::TokenExpired,
        _ => ApiError::InvalidToken,
    })?;

    if data.claims.token_type != TokenType::Refresh {
        return Err(ApiError::InvalidToken);
    }
    Ok(data.claims)
}

pub fn generate_token() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
