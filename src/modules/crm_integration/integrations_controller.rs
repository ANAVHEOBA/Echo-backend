use axum::{
    extract::{Query, State},
    response::{Redirect, IntoResponse},
};
use std::sync::Arc;
use serde::Deserialize;
use reqwest::Client;
use std::collections::HashMap;

use crate::AppState;
use crate::errors::ApiError;

#[derive(Deserialize)]
pub struct ZoomCallbackParams {
    code: String,
}

pub async fn connect_zoom(
    State(state): State<Arc<AppState>>,
) -> Result<Redirect, ApiError> {
    let client_id = &state.config.zoom_client_id;
    let redirect_uri = format!("https://{}/api/integrations/zoom/callback", state.config.host); // Or hardcode/env var
    
    // Construct Zoom Authorization URL
    let url = format!(
        "https://zoom.us/oauth/authorize?response_type=code&client_id={}&redirect_uri={}",
        client_id, redirect_uri
    );
    
    Ok(Redirect::to(&url))
}

pub async fn zoom_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ZoomCallbackParams>,
) -> Result<impl IntoResponse, ApiError> {
    let client = Client::new();
    let token_url = "https://zoom.us/oauth/token";
    
    // Determine Redirect URI based on host (local vs prod)
    // NOTE: This must match what you put in Zoom Marketplace exactly
    let redirect_uri = if state.config.host.contains("render") {
         format!("https://{}/api/integrations/zoom/callback", state.config.host)
    } else {
         // Fallback or dev default
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

    let token_data: serde_json::Value = response.json().await
        .map_err(|e| ApiError::InternalError(format!("Failed to parse Zoom response: {}", e)))?;

    // TODO: Store tokens in DB (linked to authenticated user)
    // For now, we just print them to confirm it works
    tracing::info!("Zoom Access Token: {:?}", token_data);

    Ok("Zoom Connected Successfully! You can close this window.")
}
