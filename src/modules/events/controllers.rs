use axum::{
    extract::{State, Path},
    http::{StatusCode, HeaderMap},
    Json,
    Extension,
    body::Bytes,
};
use std::sync::Arc;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex;
use uuid::Uuid;
use chrono::Utc;
use validator::Validate;

use crate::AppState;
use crate::modules::auth::model::Claims;
use super::schemas::{
    SlackWebhookPayload,
    GmailPushPayload, ZoomWebhookPayload, GenericWebhookPayload,
    EventFilter, EventResponse, EventStatsResponse,
    CreateSubscriptionRequest, SubscriptionResponse,
};
use super::models::{Event, WebhookSubscription};
use super::crud;

// =============================================================================
// SLACK WEBHOOK CONTROLLER
// =============================================================================

pub async fn slack_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let body_str = std::str::from_utf8(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    let payload: SlackWebhookPayload = serde_json::from_str(body_str)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    tracing::info!("Received Slack webhook: type={}", payload.event_type);

    if payload.event_type == "url_verification" {
        if let Some(challenge) = payload.challenge {
            return Ok(Json(serde_json::json!({
                "challenge": challenge
            })));
        }
        return Err(StatusCode::BAD_REQUEST);
    }

    let timestamp = headers.get("X-Slack-Request-Timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let signature = headers.get("X-Slack-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let secret = state.config.slack_signing_secret();
    let basestring = format!("v0:{}:{}", timestamp, body_str);

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    mac.update(basestring.as_bytes());
    let calculated_signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

    if signature != calculated_signature {
        tracing::warn!("Invalid Slack signature. Expected: {}, Got: {}", calculated_signature, signature);
        return Err(StatusCode::UNAUTHORIZED);
    }

    if payload.event_type == "event_callback" {
        if let Some(ref event_data) = payload.event {
            let event_type = event_data.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();

            let event = Event::new(
                event_type,
                "slack".to_string(),
                payload.event_id.clone(),
                serde_json::json!(payload),
            );

            match crud::create_event(&state.pool, event).await {
                Ok(Some(stored_event)) => {
                    let job = crate::services::queue::Job::new(
                        "event.process.slack",
                        serde_json::json!({
                            "event_id": stored_event.id.to_string()
                        })
                    );
                    
                    let queue = crate::services::queue::Queue::new(state.redis.clone());
                    let _ = queue.enqueue(&job).await;
                    
                    return Ok(Json(serde_json::json!({ "ok": true })));
                }
                Ok(None) => {
                    // Duplicate event (already processed)
                    return Ok(Json(serde_json::json!({ "ok": true, "duplicate": true })));
                }
                Err(e) => {
                    tracing::error!("Failed to store Slack event: {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

// =============================================================================
// GMAIL WEBHOOK CONTROLLER
// =============================================================================

pub async fn gmail_webhook(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GmailPushPayload>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Received Gmail push notification");

    let _decoded = match BASE64.decode(&payload.message.data) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Failed to decode Gmail push data: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let event = Event::new(
        "email_received".to_string(),
        "gmail".to_string(),
        Some(payload.message.message_id.clone()),
        serde_json::json!(payload),
    );

    match crud::create_event(&state.pool, event).await {
        Ok(Some(stored_event)) => {
            tracing::info!("Stored Gmail event: {}", stored_event.id);

            let job = crate::services::queue::Job::new(
                "event.process.gmail",
                serde_json::json!({
                    "event_id": stored_event.id.to_string(),
                })
            );
            
            let queue = crate::services::queue::Queue::new(state.redis.clone());
            let _ = queue.enqueue(&job).await;

            Ok(StatusCode::OK)
        }
        Ok(None) => {
            tracing::warn!("Duplicate Gmail event ignored");
            Ok(StatusCode::OK)
        }
        Err(e) => {
            tracing::error!("Failed to store Gmail event: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// =============================================================================
// ZOOM WEBHOOK CONTROLLER
// =============================================================================

pub async fn zoom_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let body_str = std::str::from_utf8(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let payload: ZoomWebhookPayload = serde_json::from_str(body_str).map_err(|_| StatusCode::BAD_REQUEST)?;
    
    tracing::info!("Received Zoom webhook: event={}", payload.event);

    if payload.event == "endpoint.url_validation" {
        let plain_token = payload.payload.get("plainToken")
            .and_then(|t| t.as_str())
            .ok_or(StatusCode::BAD_REQUEST)?;

        let secret = state.config.zoom_webhook_secret();

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        mac.update(plain_token.as_bytes());
        let result = mac.finalize();
        let hash = hex::encode(result.into_bytes());

        return Ok(Json(serde_json::json!({
            "plainToken": plain_token,
            "encryptedToken": hash
        })));
    }

    let signature = headers.get("x-zm-signature")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("authorization").and_then(|v| v.to_str().ok()));

    if let Some(sig) = signature {
         let timestamp = headers.get("x-zm-request-timestamp")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""); 

        let secret = state.config.zoom_webhook_secret();
        let message = format!("v0:{}:{}", timestamp, body_str);

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        mac.update(message.as_bytes());
        let calculated_signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        if sig != calculated_signature {
             return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let external_id = payload.payload.get("object")
        .and_then(|o| o.get("uuid"))
        .and_then(|u| u.as_str())
        .map(|s| s.to_string());

    let event = Event::new(
        payload.event.clone(),
        "zoom".to_string(),
        external_id,
        serde_json::json!(payload),
    );

    match crud::create_event(&state.pool, event).await {
        Ok(Some(stored_event)) => {
             let job = crate::services::queue::Job::new(
                "event.process.zoom",
                serde_json::json!({
                    "event_id": stored_event.id.to_string()
                })
            );
            
            let queue = crate::services::queue::Queue::new(state.redis.clone());
            let _ = queue.enqueue(&job).await;
            
            Ok(Json(serde_json::json!({ "status": "processed" })))
        },
        Ok(None) => Ok(Json(serde_json::json!({ "status": "duplicate" }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// =============================================================================
// GENERIC WEBHOOK CONTROLLER
// =============================================================================

pub async fn generic_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<GenericWebhookPayload>,
) -> Result<StatusCode, StatusCode> {
    let signature = headers.get("X-Webhook-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if signature != state.config.generic_webhook_secret() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let event = Event::new(
        payload.event.clone(),
        "generic".to_string(),
        None,
        serde_json::json!(payload),
    );

    match crud::create_event(&state.pool, event).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// =============================================================================
// EVENT MANAGEMENT CONTROLLERS
// =============================================================================

pub async fn list_events(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(filter): axum::extract::Query<EventFilter>,
) -> Result<Json<Vec<EventResponse>>, StatusCode> {
    match crud::list_events(&state.pool, filter).await {
        Ok(events) => {
            let response: Vec<EventResponse> = events.into_iter().map(|e| EventResponse {
                id: e.id.to_string(),
                event_type: e.event_type,
                source: e.source,
                external_id: e.external_id,
                payload: e.payload,
                processed_at: e.processed_at.map(|dt| dt.to_rfc3339()),
                created_at: e.created_at.to_rfc3339(),
            }).collect();
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_event(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<EventResponse>, StatusCode> {
    match crud::get_event_by_id(&state.pool, &id).await {
        Ok(Some(e)) => {
            let response = EventResponse {
                id: e.id.to_string(),
                event_type: e.event_type,
                source: e.source,
                external_id: e.external_id,
                payload: e.payload,
                processed_at: e.processed_at.map(|dt| dt.to_rfc3339()),
                created_at: e.created_at.to_rfc3339(),
            };
            Ok(Json(response))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn replay_event(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let event = match crud::get_event_by_id(&state.pool, &id).await {
        Ok(Some(e)) => e,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let job_name = match event.source.as_str() {
        "slack" => "event.process.slack",
        "gmail" => "event.process.gmail",
        "zoom" => "event.process.zoom",
        _ => "event.process.generic",
    };

    let job = crate::services::queue::Job::new(
        job_name,
        serde_json::json!({
            "event_id": event.id.to_string(),
            "replay": true
        })
    );
    
    let queue = crate::services::queue::Queue::new(state.redis.clone());
    if let Err(_e) = queue.enqueue(&job).await {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(serde_json::json!({ 
        "message": "Event replay enqueued", 
        "event_id": id 
    })))
}

pub async fn get_event_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<EventStatsResponse>, StatusCode> {
    match crud::get_event_stats(&state.pool).await {
        Ok((total, processed, pending, failed)) => {
            Ok(Json(EventStatsResponse {
                total_events: total,
                processed_events: processed,
                pending_events: pending,
                failed_events: failed,
            }))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// =============================================================================
// SUBSCRIPTION CONTROLLERS
// =============================================================================

pub async fn create_subscription(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateSubscriptionRequest>,
) -> Result<(StatusCode, Json<SubscriptionResponse>), StatusCode> {
    
    // Validate request
    if let Err(_) = payload.validate() {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    if !["slack", "gmail", "zoom", "generic"].contains(&payload.platform.as_str()) {
         return Err(StatusCode::UNPROCESSABLE_ENTITY); // Or BAD_REQUEST, but test expects validation error
    }

    let subscription = WebhookSubscription {
        id: Uuid::new_v4(),
        user_id: claims.sub,
        platform: payload.platform.clone(),
        webhook_url: payload.webhook_url.clone(),
        secret: payload.secret,
        event_types: payload.event_types.map(|t| serde_json::to_value(t).unwrap_or(serde_json::Value::Null)),
        active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    match crud::create_subscription(&state.pool, subscription).await {
        Ok(sub) => Ok((StatusCode::CREATED, Json(SubscriptionResponse {
            id: sub.id.to_string(),
            platform: sub.platform,
            webhook_url: sub.webhook_url,
            active: sub.active,
            created_at: sub.created_at.to_rfc3339(),
        }))),
        Err(e) => {
            tracing::error!("Failed to create subscription: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn list_subscriptions(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<SubscriptionResponse>>, StatusCode> {
    match crud::list_subscriptions(&state.pool, &claims.sub).await {
        Ok(subs) => {
            let response = subs.into_iter().map(|s| SubscriptionResponse {
                id: s.id.to_string(),
                platform: s.platform,
                webhook_url: s.webhook_url,
                active: s.active,
                created_at: s.created_at.to_rfc3339(),
            }).collect();
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to list subscriptions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_subscription(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    match crud::delete_subscription(&state.pool, &id, &claims.sub).await {
        Ok(0) => Err(StatusCode::NOT_FOUND),
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            tracing::error!("Failed to delete subscription: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}