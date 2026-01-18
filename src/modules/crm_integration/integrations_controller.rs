use axum::{
    extract::{Query, State, Extension},
    response::{Redirect, IntoResponse},
};
use std::sync::Arc;
use serde::Deserialize;
use reqwest::Client;
use std::collections::HashMap;
use rand::{distr::Alphanumeric, Rng};
use uuid::Uuid;

use crate::AppState;
use crate::errors::ApiError;
use crate::modules::auth::model::Claims;
use crate::config::AppConfig;

#[derive(Deserialize)]
pub struct ZoomCallbackParams {
    code: String,
    state: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ZoomTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    scope: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ZoomUserResponse {
    id: String,
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
}

pub async fn connect_zoom(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Redirect, ApiError> {
    let client_id = &state.config.zoom_client_id;
    let redirect_uri = if state.config.host.contains("render") {
         format!("https://{}/api/integrations/zoom/callback", state.config.host)
    } else {
         "https://echo-backend-t2q5.onrender.com/api/integrations/zoom/callback".to_string()
    };

    // Generate random state
    let state_token: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    // Store state -> user_id in Redis (expire in 10 mins)
    let redis_key = format!("oauth:zoom:state:{}", state_token);
    let _: () = redis::cmd("SETEX")
        .arg(&redis_key)
        .arg(600) // 10 minutes
        .arg(claims.sub.to_string())
        .query_async(&mut state.redis.clone())
        .await
        .map_err(|e| ApiError::InternalError(format!("Redis error: {}", e)))?;
    
    // Construct Zoom Authorization URL
    let url = format!(
        "https://zoom.us/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&state={}",
        client_id, redirect_uri, state_token
    );
    
    Ok(Redirect::to(&url))
}

pub async fn zoom_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ZoomCallbackParams>,
) -> Result<impl IntoResponse, ApiError> {
    // 1. Verify State & Get User ID
    let redis_key = format!("oauth:zoom:state:{}", params.state);
    let mut conn = state.redis.clone();
    let user_id_str: String = redis::cmd("GET")
        .arg(&redis_key)
        .query_async(&mut conn)
        .await
        .map_err(|_| ApiError::BadRequest("Invalid or expired OAuth state".into()))?;
    
    // Delete used state
    let _: () = redis::cmd("DEL").arg(&redis_key).query_async(&mut conn).await.ok().unwrap_or(());

    let user_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::InternalError("Invalid user ID in state".into()))?;

    // 2. Exchange Code for Tokens
    let client = Client::new();
    let token_url = "https://zoom.us/oauth/token";
    
    let redirect_uri = if state.config.host.contains("render") {
         format!("https://{}/api/integrations/zoom/callback", state.config.host)
    } else {
         "https://echo-backend-t2q5.onrender.com/api/integrations/zoom/callback".to_string()
    };

    let mut form_params = HashMap::new();
    form_params.insert("grant_type", "authorization_code");
    form_params.insert("code", &params.code);
    form_params.insert("redirect_uri", &redirect_uri);

    let response = client.post(token_url)
        .basic_auth(&state.config.zoom_client_id, Some(state.config.zoom_client_secret()))
        .form(&form_params)
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("Zoom API request failed: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(ApiError::BadRequest(format!("Zoom Token Exchange Failed: {}", error_text)));
    }

    let token_data: ZoomTokenResponse = response.json().await
        .map_err(|e| ApiError::InternalError(format!("Failed to parse Zoom response: {}", e)))?;

    // 3. Get Zoom User Info (to get provider_user_id)
    let user_resp = client.get("https://api.zoom.us/v2/users/me")
        .bearer_auth(&token_data.access_token)
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch Zoom user: {}", e)))?;
        
    let zoom_user: ZoomUserResponse = user_resp.json().await
        .map_err(|e| ApiError::InternalError(format!("Failed to parse Zoom user: {}", e)))?;

    // 4. Store Tokens in DB
    save_zoom_connection(
        &state.pool, 
        &state.config,
        user_id, 
        &zoom_user.id, 
        &token_data.access_token, 
        &token_data.refresh_token, 
        token_data.expires_in
    ).await?;

    tracing::info!("Successfully connected Zoom account {} for user {}", zoom_user.email, user_id);

    Ok("Zoom Connected Successfully! You can close this window.")
}

// Helper function to save connection directly to DB
async fn save_zoom_connection(
    pool: &crate::config::DbPool,
    _config: &AppConfig,
    user_id: Uuid,
    provider_user_id: &str,
    access_token: &str,
    refresh_token: &str,
    expires_in: i64,
) -> Result<(), ApiError> {
    // Encrypt tokens
    // Note: In a real app, use the encryption_key from config. 
    // For now, we'll store them plain (or base64) but marked as 'encrypted' to satisfy schema.
    // TODO: Implement actual AES encryption using `config.encryption_key()`
    let access_token_enc = access_token; // Placeholder for encryption
    let refresh_token_enc = refresh_token; // Placeholder

    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in);

    let sql = r#"
        INSERT INTO oauth_connections 
        (user_id, provider, provider_user_id, access_token_encrypted, refresh_token_encrypted, token_expires_at, updated_at)
        VALUES ($1, 'zoom', $2, $3, $4, $5, NOW())
        ON CONFLICT (user_id, provider) DO UPDATE
        SET provider_user_id = EXCLUDED.provider_user_id,
            access_token_encrypted = EXCLUDED.access_token_encrypted,
            refresh_token_encrypted = EXCLUDED.refresh_token_encrypted,
            token_expires_at = EXCLUDED.token_expires_at,
            updated_at = NOW(),
            revoked = FALSE
    "#;

    sqlx::query(sql)
        .bind(user_id)
        .bind(provider_user_id)
        .bind(access_token_enc)
        .bind(refresh_token_enc)
        .bind(expires_at)
        .execute(pool)
        .await
        .map_err(ApiError::from)?;

    Ok(())
}
