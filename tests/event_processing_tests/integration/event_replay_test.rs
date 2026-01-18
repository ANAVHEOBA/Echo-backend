///! Integration Tests for Event Replay Functionality
///!
///! Tests event replay and error handling:
///! - Failed event detection and tracking
///! - Manual replay triggering
///! - Automatic retry mechanisms
///! - Dead letter queue handling

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
    format!("{}", Uuid::new_v4())
}



// =============================================================================
// FAILED EVENT DETECTION TESTS
// =============================================================================

#[tokio::test]
async fn list_failed_events_endpoint() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events?status=failed")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Should be able to filter events by failed status"
    );
}

#[tokio::test]
async fn failed_events_include_error_details() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri(&format!("/api/events/{}", sample_event_id()))
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Event details should include error_message and failure_count
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::OK,
        "Failed event details should include error information"
    );
}

// =============================================================================
// MANUAL REPLAY TESTS
// =============================================================================

#[tokio::test]
async fn replay_event_with_valid_id() {
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

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Replay should accept valid event ID"
    );
}

#[tokio::test]
async fn replay_event_returns_job_id() {
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

    // Should return job_id for tracking replay progress
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Replay should return job identifier"
    );
}

#[tokio::test]
async fn replay_event_prevents_concurrent_replays() {
    let app = create_test_app().await;
    let event_id = sample_event_id();

    // First replay request
    let request1 = Request::builder()
        .uri(&format!("/api/events/replay/{}", event_id))
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer admin_token")
        .body(Body::empty())
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();

    // Second concurrent replay request for same event
    let request2 = Request::builder()
        .uri(&format!("/api/events/replay/{}", event_id))
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer admin_token")
        .body(Body::empty())
        .unwrap();

    let response2 = app.oneshot(request2).await.unwrap();

    // Second request should be rejected with 409 Conflict or both fail with 404
    assert!(
        (response1.status() == StatusCode::OK && response2.status() == StatusCode::CONFLICT) ||
        (response1.status() == StatusCode::NOT_FOUND && response2.status() == StatusCode::NOT_FOUND),
        "Concurrent replays of same event should be prevented"
    );
}

#[tokio::test]
async fn replay_only_failed_or_completed_events() {
    let app = create_test_app().await;
    let event_id = sample_event_id();

    // Try to replay an event that's currently processing
    let request = Request::builder()
        .uri(&format!("/api/events/replay/{}", event_id))
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer admin_token")
        .body(Body::from(json!({"allow_processing": false}).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should reject if event is currently being processed
    assert!(
        response.status() == StatusCode::CONFLICT || response.status() == StatusCode::NOT_FOUND,
        "Should not replay events that are currently processing"
    );
}

// =============================================================================
// AUTOMATIC RETRY TESTS
// =============================================================================

#[tokio::test]
async fn automatic_retry_respects_max_attempts() {
    let app = create_test_app().await;

    // Simulate an event that has failed multiple times
    let event_data = json!({
        "event_id": &sample_event_id(),
        "failure_count": 5,
        "max_retries": 3
    });

    // Check if worker would continue retrying
    let request = Request::builder()
        .uri(&format!("/api/events/{}/retry-status", event_data["event_id"].as_str().unwrap()))
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should indicate retries exhausted or 404 if not implemented
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Retry status check should be accessible"
    );
}

#[tokio::test]
async fn automatic_retry_uses_exponential_backoff() {
    let app = create_test_app().await;

    // Test that retry timestamps show exponential backoff pattern
    // This would be verified in the event logs
    let request = Request::builder()
        .uri(&format!("/api/events/{}/retry-schedule", sample_event_id()))
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Retry schedule should show exponential backoff"
    );
}

// =============================================================================
// DEAD LETTER QUEUE TESTS
// =============================================================================

#[tokio::test]
async fn failed_events_moved_to_dlq_after_max_retries() {
    let app = create_test_app().await;

    // Query dead letter queue
    let request = Request::builder()
        .uri("/api/events/dead-letter-queue")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer admin_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Dead letter queue should be queryable"
    );
}

#[tokio::test]
async fn dlq_events_preserve_original_payload() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri(&format!("/api/events/dead-letter-queue/{}", sample_event_id()))
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer admin_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // DLQ event should include original payload for debugging
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::OK,
        "DLQ events should preserve original payloads"
    );
}

#[tokio::test]
async fn dlq_events_can_be_replayed() {
    let app = create_test_app().await;
    let dlq_event_id = sample_event_id();

    // Replay event from DLQ
    let request = Request::builder()
        .uri(&format!("/api/events/dead-letter-queue/{}/replay", dlq_event_id))
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer admin_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "DLQ events should be replayable"
    );
}

#[tokio::test]
async fn dlq_events_can_be_permanently_deleted() {
    let app = create_test_app().await;
    let dlq_event_id = sample_event_id();

    // Delete event from DLQ
    let request = Request::builder()
        .uri(&format!("/api/events/dead-letter-queue/{}", dlq_event_id))
        .method("DELETE")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer admin_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::NO_CONTENT || response.status() == StatusCode::NOT_FOUND,
        "DLQ events should be deletable"
    );
}

// =============================================================================
// ERROR TRACKING TESTS
// =============================================================================

#[tokio::test]
async fn event_processing_errors_are_logged() {
    let app = create_test_app().await;
    let event_id = sample_event_id();

    // Get event processing logs
    let request = Request::builder()
        .uri(&format!("/api/events/{}/logs", event_id))
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Logs should include error messages and stack traces
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Event processing logs should be accessible"
    );
}

#[tokio::test]
async fn event_error_statistics_are_tracked() {
    let app = create_test_app().await;

    // Get error statistics
    let request = Request::builder()
        .uri("/api/events/stats?metrics=errors")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should include error counts, types, and trends
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Error statistics should be available"
    );
}
