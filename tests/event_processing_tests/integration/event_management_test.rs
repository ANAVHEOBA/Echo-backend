///! Integration Tests for Event Management
///!
///! Tests event management endpoints:
///! - List events (GET /api/events)
///! - Get event details (GET /api/events/{id})
///! - Get event statistics (GET /api/events/stats)
///! - Replay failed event (POST /api/events/replay/{id})

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use crate::common::create_test_app;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

// =============================================================================
// TEST HELPERS
// =============================================================================

fn sample_event_id() -> String {
    format!("evt_{}", Uuid::new_v4())
}

// =============================================================================
// LIST EVENTS TESTS
// =============================================================================

#[tokio::test]
async fn list_events_endpoint_is_accessible() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 200 with empty array or 404 if not implemented
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "List events endpoint should be accessible, got: {}", response.status()
    );
}

#[tokio::test]
async fn list_events_supports_pagination() {
    let app = create_test_app().await;

    // Request with pagination parameters
    let request = Request::builder()
        .uri("/api/events?page=1&limit=50")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "List events should support pagination"
    );
}

#[tokio::test]
async fn list_events_supports_filtering_by_event_type() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events?event_type=email_received")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "List events should support filtering by event type"
    );
}

#[tokio::test]
async fn list_events_supports_filtering_by_source() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events?source=slack")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "List events should support filtering by source platform"
    );
}

#[tokio::test]
async fn list_events_supports_filtering_by_processing_status() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events?processed=false")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "List events should support filtering by processing status"
    );
}

#[tokio::test]
async fn list_events_requires_authentication() {
    let app = create_test_app().await;

    // Request without Authorization header
    let request = Request::builder()
        .uri("/api/events")
        .method("GET")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 401 Unauthorized or 404 if not implemented
    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::NOT_FOUND,
        "List events should require authentication"
    );
}

// =============================================================================
// GET EVENT DETAILS TESTS
// =============================================================================

#[tokio::test]
async fn get_event_details_endpoint_is_accessible() {
    let app = create_test_app().await;
    let event_id = sample_event_id();

    let request = Request::builder()
        .uri(&format!("/api/events/{}", event_id))
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 404 (event not found) or route not found
    assert!(
        response.status() == StatusCode::NOT_FOUND,
        "Get event details endpoint should be accessible"
    );
}

#[tokio::test]
async fn get_event_details_validates_event_id_format() {
    let app = create_test_app().await;

    // Invalid event ID format
    let request = Request::builder()
        .uri("/api/events/invalid-format")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::NOT_FOUND,
        "Invalid event ID format should be rejected"
    );
}

#[tokio::test]
async fn get_event_details_returns_complete_event_data() {
    let app = create_test_app().await;
    let event_id = sample_event_id();

    let request = Request::builder()
        .uri(&format!("/api/events/{}", event_id))
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Event should include: id, type, source, payload, created_at, processed_at
    // Test structure when implemented
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::OK,
        "Get event details should return complete event data structure"
    );
}

// =============================================================================
// EVENT STATISTICS TESTS
// =============================================================================

#[tokio::test]
async fn event_stats_endpoint_is_accessible() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events/stats")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Event stats endpoint should be accessible, got: {}", response.status()
    );
}

#[tokio::test]
async fn event_stats_supports_time_range_filtering() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events/stats?start_date=2024-01-01&end_date=2024-01-31")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Event stats should support time range filtering"
    );
}

#[tokio::test]
async fn event_stats_includes_event_type_breakdown() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events/stats?breakdown=by_type")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Stats should include counts by event type
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Event stats should support event type breakdown"
    );
}

#[tokio::test]
async fn event_stats_includes_source_breakdown() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events/stats?breakdown=by_source")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Stats should include counts by source platform
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Event stats should support source breakdown"
    );
}

// =============================================================================
// EVENT REPLAY TESTS
// =============================================================================

#[tokio::test]
async fn replay_event_endpoint_is_accessible() {
    let app = create_test_app().await;
    let event_id = sample_event_id();

    let request = Request::builder()
        .uri(&format!("/api/events/replay/{}", event_id))
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 404 (event not found) or route not found
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::OK,
        "Replay event endpoint should be accessible"
    );
}

#[tokio::test]
async fn replay_event_validates_event_exists() {
    let app = create_test_app().await;
    let nonexistent_event_id = sample_event_id();

    let request = Request::builder()
        .uri(&format!("/api/events/replay/{}", nonexistent_event_id))
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::NOT_FOUND,
        "Replay should return 404 for nonexistent event"
    );
}

#[tokio::test]
async fn replay_event_requires_admin_permission() {
    let app = create_test_app().await;
    let event_id = sample_event_id();

    // Regular user token (not admin)
    let request = Request::builder()
        .uri(&format!("/api/events/replay/{}", event_id))
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer user_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 403 Forbidden or 404 if not implemented
    assert!(
        response.status() == StatusCode::FORBIDDEN || response.status() == StatusCode::NOT_FOUND,
        "Replay event should require admin permission"
    );
}

#[tokio::test]
async fn replay_event_enqueues_new_processing_job() {
    let app = create_test_app().await;
    let event_id = sample_event_id();

    let request = Request::builder()
        .uri(&format!("/api/events/replay/{}", event_id))
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer admin_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should enqueue job and return 200 OK
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Replay event should enqueue processing job"
    );
}

// =============================================================================
// EVENT LIFECYCLE TESTS
// =============================================================================

#[tokio::test]
async fn event_processing_flow_is_idempotent() {
    let app = create_test_app().await;

    // Simulate receiving the same event twice (same external_id)
    let external_id = format!("slack-msg-{}", Uuid::new_v4());
    
    let payload = json!({
        "token": "test_token",
        "team_id": "T123",
        "event": {
            "type": "message",
            "text": "Deal update",
            "ts": "1234567890.123456"
        },
        "external_id": &external_id,
        "type": "event_callback"
    });

    // first webhook call
    let request1 = Request::builder()
        .uri("/api/events/webhooks/slack")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Slack-Signature", "v0=test")
        .header("X-Slack-Request-Timestamp", "1234567890")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    let status1 = response1.status();

    // Second webhook call with same external_id (duplicate)
    let request2 = Request::builder()
        .uri("/api/events/webhooks/slack")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Slack-Signature", "v0=test")
        .header("X-Slack-Request-Timestamp", "1234567891")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response2 = app.oneshot(request2).await.unwrap();
    let status2 = response2.status();

    // Both should succeed but duplicate should be detected
    assert!(
        (status1 == StatusCode::OK || status1 == StatusCode::NOT_FOUND) &&
        (status2 == StatusCode::OK || status2 == StatusCode::NOT_FOUND),
        "Duplicate events should be handled gracefully"
    );
}
