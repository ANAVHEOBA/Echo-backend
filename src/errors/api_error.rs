use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Account locked until {0}")]
    AccountLocked(String),

    #[error("Account deleted")]
    AccountDeleted,

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("User not found")]
    UserNotFound,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Token already used")]
    TokenAlreadyUsed,

    #[error("Token revoked")]
    TokenRevoked,

    #[error("Invalid password: {0}")]
    InvalidPassword(String),

    #[error("API key limit reached")]
    ApiKeyLimitReached,

    #[error("API key not found")]
    ApiKeyNotFound,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Database error")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Internal error")]
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, code) = match &self {
            ApiError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"),
            ApiError::AccountLocked(_) => (StatusCode::FORBIDDEN, "ACCOUNT_LOCKED"),
            ApiError::AccountDeleted => (StatusCode::FORBIDDEN, "ACCOUNT_DELETED"),
            ApiError::UserAlreadyExists => (StatusCode::CONFLICT, "USER_EXISTS"),
            ApiError::UserNotFound => (StatusCode::NOT_FOUND, "USER_NOT_FOUND"),
            ApiError::InvalidToken => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN"),
            ApiError::TokenExpired => (StatusCode::UNAUTHORIZED, "TOKEN_EXPIRED"),
            ApiError::TokenAlreadyUsed => (StatusCode::BAD_REQUEST, "TOKEN_USED"),
            ApiError::TokenRevoked => (StatusCode::UNAUTHORIZED, "TOKEN_REVOKED"),
            ApiError::InvalidPassword(_) => (StatusCode::BAD_REQUEST, "INVALID_PASSWORD"),
            ApiError::ApiKeyLimitReached => (StatusCode::FORBIDDEN, "API_KEY_LIMIT"),
            ApiError::ApiKeyNotFound => (StatusCode::NOT_FOUND, "API_KEY_NOT_FOUND"),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            ApiError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR"),
            ApiError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        let body = Json(ErrorBody {
            code: code.to_string(),
            message: self.to_string(),
        });

        (status, body).into_response()
    }
}
