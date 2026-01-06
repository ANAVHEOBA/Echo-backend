use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;
use validator::Validate;

use crate::AppState;
use crate::errors::ApiError;
use super::crud::UserCrud;
use super::schema::*;
use super::services::validate_access_token;

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), ApiError> {
    req.validate().map_err(|e| ApiError::InvalidPassword(e.to_string()))?;

    let crud = UserCrud::new(&state.pool, &state.config);

    if crud.email_exists(&req.email).await? {
        return Err(ApiError::UserAlreadyExists);
    }

    let user = crud.create_user(&req.email, &req.password, req.first_name.as_deref(), req.last_name.as_deref()).await?;

    Ok((StatusCode::CREATED, Json(RegisterResponse {
        user: user.to_response(),
        message: "Registration successful",
    })))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let crud = UserCrud::new(&state.pool, &state.config);
    let result = crud.login(&req.email, &req.password).await?;

    Ok(Json(LoginResponse {
        access_token: result.access_token,
        refresh_token: result.refresh_token,
        token_type: "Bearer",
        expires_in: state.config.access_token_expiry_secs,
    }))
}

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, ApiError> {
    let crud = UserCrud::new(&state.pool, &state.config);
    let result = crud.refresh_tokens(&req.refresh_token).await?;

    Ok(Json(RefreshTokenResponse {
        access_token: result.access_token,
        refresh_token: result.refresh_token,
        token_type: "Bearer",
        expires_in: state.config.access_token_expiry_secs,
    }))
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    let crud = UserCrud::new(&state.pool, &state.config);
    crud.logout(&req.refresh_token).await?;
    Ok(Json(MessageResponse { message: "Logged out" }))
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
) -> Result<Json<UserResponse>, ApiError> {
    let token = req.headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError::InvalidToken)?;

    let claims = validate_access_token(token, &state.config)?;

    let crud = UserCrud::new(&state.pool, &state.config);
    let user = crud.find_by_id(&claims.sub).await?.ok_or(ApiError::UserNotFound)?;

    if user.is_deleted() {
        return Err(ApiError::AccountDeleted);
    }

    Ok(Json(user.to_response()))
}
