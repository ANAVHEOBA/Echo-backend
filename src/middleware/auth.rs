use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::modules::auth::{crud::UserCrud, services::validate_access_token};

#[derive(Clone)]
pub struct AuthState {
    pub config: Arc<AppConfig>,
    pub pool: PgPool,
}

#[derive(Serialize)]
struct AuthErrorBody {
    code: &'static str,
    message: &'static str,
}

fn unauthorized(msg: &'static str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(AuthErrorBody { code: "UNAUTHORIZED", message: msg }),
    )
        .into_response()
}

pub async fn require_auth(mut req: Request<Body>, next: Next) -> Response {
    // Get auth state from extensions (must be added when building router)
    let auth_state = match req.extensions().get::<AuthState>().cloned() {
        Some(s) => s,
        None => return unauthorized("Auth not configured"),
    };

    // Extract token
    let token = match req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        Some(t) => t.to_string(),
        None => return unauthorized("Missing token"),
    };

    // Validate JWT
    let claims = match validate_access_token(&token, &auth_state.config) {
        Ok(c) => c,
        Err(_) => return unauthorized("Invalid token"),
    };

    // Check user exists and is valid
    let crud = UserCrud::new(&auth_state.pool, &auth_state.config);
    let user = match crud.find_by_id(&claims.sub).await {
        Ok(Some(u)) => u,
        Ok(None) => return unauthorized("User not found"),
        Err(_) => return unauthorized("Auth check failed"),
    };

    if user.is_deleted() {
        return unauthorized("Account deleted");
    }

    if user.is_locked() {
        return unauthorized("Account locked");
    }

    // Add claims to request
    req.extensions_mut().insert(claims);
    next.run(req).await
}
