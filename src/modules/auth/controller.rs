use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    response::{Redirect, IntoResponse},
    Json,
};
use std::sync::Arc;
use validator::Validate;
use oauth2::{TokenResponse, CsrfToken, PkceCodeVerifier};

use crate::AppState;
use crate::errors::ApiError;
use super::crud::UserCrud;
use super::schema::*;
use super::services::validate_access_token;
use super::services::oauth::GoogleOAuthService;

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

// =============================================================================
// OAUTH HANDLERS
// =============================================================================

#[derive(serde::Deserialize)]
pub struct OAuthCallbackParams {
    code: String,
    state: String,
}

pub async fn oauth_authorize(
    State(state): State<Arc<AppState>>,
    Path(provider): Path<String>,
) -> Result<Redirect, ApiError> {
    // For now, only Google is supported
    if provider != "google" {
        return Err(ApiError::BadRequest("Only 'google' provider is currently supported".into()));
    }

    let oauth_service = GoogleOAuthService::new(&state.config)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let (auth_url, csrf_token, pkce_verifier): (url::Url, CsrfToken, PkceCodeVerifier) = oauth_service.get_authorization_url();

    let redis_key = format!("oauth:state:{}", csrf_token.secret());
    let _: () = redis::cmd("SETEX")
        .arg(&redis_key)
        .arg(600) // 10 minutes
        .arg(pkce_verifier.secret())
        .query_async(&mut state.redis.clone())
        .await
        .map_err(|e| ApiError::InternalError(format!("Redis error: {}", e)))?;

    Ok(Redirect::to(auth_url.as_str()))
}

pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Path(provider): Path<String>,
    Query(params): Query<OAuthCallbackParams>,
) -> Result<impl IntoResponse, ApiError> {
    if provider != "google" {
        return Err(ApiError::BadRequest("Only 'google' provider is currently supported".into()));
    }

    // 1. Verify State & Retrieve PKCE Verifier
    let redis_key = format!("oauth:state:{}", params.state);
    let mut conn = state.redis.clone();
    let pkce_verifier_secret: String = redis::cmd("GET")
        .arg(&redis_key)
        .query_async(&mut conn)
        .await
        .map_err(|_| ApiError::BadRequest("Invalid or expired OAuth state".into()))?;

    // Delete used state
    let _: () = redis::cmd("DEL").arg(&redis_key).query_async(&mut conn).await.ok().unwrap_or(());

    // 2. Exchange Code for Tokens
    let oauth_service = GoogleOAuthService::new(&state.config)
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let token_response = oauth_service.exchange_code(params.code, pkce_verifier_secret).await
        .map_err(|e| ApiError::BadRequest(format!("Failed to exchange code: {}", e)))?;

    let google_access_token = token_response.access_token().secret();
    let google_refresh_token = token_response.refresh_token().map(|t| t.secret().to_string());
    let google_expires_in = token_response.expires_in().map(|d| d.as_secs());

    // 3. Get User Info from Google
    let user_info = oauth_service.get_user_info(google_access_token).await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch user info: {}", e)))?;

    // 4. Create or Get User
    let crud = UserCrud::new(&state.pool, &state.config);
    
    // Check if user exists by email
    let user = if crud.email_exists(&user_info.email).await? {
        // Find existing user - using runtime query to avoid compile-time DB issues
        sqlx::query_as::<_, crate::modules::auth::model::User>(
            "SELECT id, email, password_hash, first_name, last_name, organization_id, role::TEXT, email_verified, email_verified_at, failed_login_attempts, locked_until, created_at, updated_at, deleted_at FROM users WHERE email = $1"
        )
        .bind(&user_info.email)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| ApiError::InternalError("Failed to fetch existing user".into()))?
    } else {
        // Create new user (Generate a random password since they use OAuth)
        let random_password = uuid::Uuid::new_v4().to_string();
        crud.create_user(
            &user_info.email,
            &random_password,
            Some(&user_info.given_name),
            Some(&user_info.family_name),
        ).await?
    };

    // 5. Store OAuth Credentials (CRITICAL for background processing)
    crud.save_oauth_connection(
        user.id,
        "google",
        google_access_token,
        google_refresh_token.as_deref(),
        google_expires_in
    ).await?;

    // 6. Generate Internal JWT Tokens
    let token_result = crud.generate_tokens_for_user(&user).await?;

    // 7. Return Tokens to Frontend (via Redirect or JSON)
    let frontend_callback = if state.config.host == "0.0.0.0" || state.config.host == "127.0.0.1" {
        "http://localhost:5173/auth/callback"
    } else {
        "https://echo-frontend-kohl.vercel.app/auth/callback"
    };

    let redirect_url = format!(
        "{}?access_token={}&refresh_token={}&expires_in={}",
        frontend_callback,
        token_result.access_token,
        token_result.refresh_token,
        state.config.access_token_expiry_secs
    );

    Ok(Redirect::to(&redirect_url))
}
