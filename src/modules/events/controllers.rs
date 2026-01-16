use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::AppState;
use super::schemas::{
    SlackWebhookPayload,
    GmailPushPayload, ZoomWebhookPayload, GenericWebhookPayload,
    EventFilter, EventResponse, EventStatsResponse,
};
use super::models::Event;
use super::crud;

// =============================================================================
// SLACK WEBHOOK CONTROLLER
// =============================================================================

pub async fn slack_webhook(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SlackWebhookPayload>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Received Slack webhook: type={}", payload.event_type);

    // Handle URL verification challenge
    if payload.event_type == "url_verification" {
        if let Some(challenge) = payload.challenge {
            tracing::info!("Responding to Slack URL verification challenge");
            return Ok(Json(serde_json::json!({
                "challenge": challenge
            })));
        }
        return Err(StatusCode::BAD_REQUEST);
    }

    // Handle event callbacks
    if payload.event_type == "event_callback" {
        if let Some(ref event_data) = payload.event {
            // Extract event type from nested event
            let event_type = event_data.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Create event record
            let event = Event::new(
                event_type,
                "slack".to_string(),
                payload.event_id.clone(),
                serde_json::json!(payload),
            );

            // Store event in database
            match crud::create_event(&state.pool, event).await {
                Ok(stored_event) => {
                    tracing::info!("Stored Slack event: {}", stored_event.id);
                    
                    // Enqueue job for processing
                    let job = crate::services::queue::Job::new(
                        "event.process.slack",
                        serde_json::json!({
                            "event_id": stored_event.id.to_string()
                        })
                    );
                    
                    let queue = crate::services::queue::Queue::new(state.redis.clone());
                    if let Err(e) = queue.enqueue(&job).await {
                        tracing::error!("Failed to enqueue processing job: {}", e);
                        // Don't fail the request even if job enqueueing fails
                    } else {
                        tracing::debug!("Enqueued processing job for event: {}", stored_event.id);
                    }
                    
                    return Ok(Json(serde_json::json!({ "ok": true })));
                }
                Err(sqlx::Error::Database(ref e)) if e.is_unique_violation() => {
                    // Duplicate event (already processed)
                    tracing::warn!("Duplicate Slack event ignored");
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

    // Decode base64 data
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
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// =============================================================================
// ZOOM WEBHOOK CONTROLLER
// =============================================================================

pub async fn zoom_webhook(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ZoomWebhookPayload>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Received Zoom webhook: event={}", payload.event);

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
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// =============================================================================
// GENERIC WEBHOOK CONTROLLER
// =============================================================================

pub async fn generic_webhook(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenericWebhookPayload>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Received generic webhook: event={}", payload.event);

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

pub async fn get_event_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<EventStatsResponse>, StatusCode> {
    match crud::get_event_stats(&state.pool).await {
        Ok((total, processed, pending)) => {
            Ok(Json(EventStatsResponse {
                total_events: total,
                processed_events: processed,
                pending_events: pending,
                failed_events: 0, // TODO: Calculate from event_logs
            }))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
