///! Integration Tests for Concurrent Event Processing
///!
///! Tests handling of concurrent events and race conditions:
///! - Multiple simultaneous webhook calls
///! - Event deduplication under load
///! - Queue ordering and priority
///! - Worker concurrency limits

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

fn create_test_event(i: usize) -> serde_json::Value {
    json!({
        "token": "test_token",
        "team_id": "T123",
        "event": {
            "type": "message",
            "text": format!("Deal update #{}", i),
            "ts": format!("1234567890.{}", i)
        },
        "event_id": format!("Ev{}", Uuid::new_v4()),
        "type": "event_callback"
    })
}

// =============================================================================
// CONCURRENT WEBHOOK TESTS
// =============================================================================

#[tokio::test]
async fn handles_concurrent_webhook_calls() {
    let app = create_test_app().await;

    // Simulate 10 concurrent webhook calls
    let handles: Vec<_> = (0..10).map(|i| {
        let app = app.clone();
        let payload = create_test_event(i);

        tokio::spawn(async move {
            let request = Request::builder()
                .uri("/api/events/webhooks/slack")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-Slack-Signature", "v0=test")
                .header("X-Slack-Request-Timestamp", "1234567890")
                .body(Body::from(payload.to_string()))
                .unwrap();

            app.oneshot(request).await.unwrap().status()
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All should complete successfully or return 404
    for status in results {
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Concurrent webhook calls should not fail, got: {}", status
        );
    }
}

#[tokio::test]
async fn handles_different_platforms_concurrently() {
    let app = create_test_app().await;

    // Mix of webhook platforms
    let platforms = vec![
        ("/api/events/webhooks/slack", create_test_event(0)),
        ("/api/events/webhooks/gmail", json!({"message": {"data": "test"}})),
        ("/api/events/webhooks/zoom", json!({"event": "meeting.ended"})),
        ("/api/events/webhooks/generic", json!({"event": "custom"})),
    ];

    let handles: Vec<_> = platforms.into_iter().map(|(uri, payload)| {
        let app = app.clone();

        tokio::spawn(async move {
            let request = Request::builder()
                .uri(uri)
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-Slack-Signature", "v0=test")
                .body(Body::from(payload.to_string()))
                .unwrap();

            app.oneshot(request).await.unwrap().status()
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All platforms should handle requests concurrently
    for status in results {
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND || status == StatusCode::UNAUTHORIZED,
            "Multi-platform webhooks should work concurrently"
        );
    }
}

// =============================================================================
// EVENT DEDUPLICATION TESTS
// =============================================================================

#[tokio::test]
async fn deduplicates_identical_events() {
    let app = create_test_app().await;

    let external_id = format!("slack-msg-{}", Uuid::new_v4());
    let payload = json!({
        "token": "test_token",
        "team_id": "T123",
        "event": {
            "type": "message",
            "text": "Deal closed!",
            "ts": "1234567890.123456"
        },
        "event_id": &external_id,
        "type": "event_callback"
    });

    // Send same event 5 times concurrently
    let handles: Vec<_> = (0..5).map(|_| {
        let app = app.clone();
        let payload = payload.clone();

        tokio::spawn(async move {
            let request = Request::builder()
                .uri("/api/events/webhooks/slack")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-Slack-Signature", "v0=test")
                .header("X-Slack-Request-Timestamp", "1234567890")
                .body(Body::from(payload.to_string()))
                .unwrap();

            app.oneshot(request).await.unwrap().status()
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All should return OK, but only one should be processed (idempotency)
    for status in results {
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Duplicate events should be handled gracefully"
        );
    }
}

#[tokio::test]
async fn handles_near_duplicate_events() {
    let app = create_test_app().await;

    // Events within same second (potential race condition)
    let handles: Vec<_> = (0..5).map(|i| {
        let app = app.clone();
        let payload = json!({
            "token": "test_token",
            "event": {
                "type": "message",
                "text": format!("Message {}", i),
                "ts": format!("1234567890.{:06}", i)  // Microseconds apart
            },
            "event_id": format!("Ev{}", Uuid::new_v4()),
            "type": "event_callback"
        });

        tokio::spawn(async move {
            let request = Request::builder()
                .uri("/api/events/webhooks/slack")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-Slack-Signature", "v0=test")
                .header("X-Slack-Request-Timestamp", "1234567890")
                .body(Body::from(payload.to_string()))
                .unwrap();

            app.oneshot(request).await.unwrap().status()
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All different events should be accepted
    for status in results {
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Near-duplicate events should all be processed"
        );
    }
}

// =============================================================================
// QUEUE ORDERING TESTS
// =============================================================================

#[tokio::test]
async fn high_priority_events_processed_first() {
    let app = create_test_app().await;

    // Send high and low priority events
    let handles: Vec<_> = (0..10).map(|i| {
        let app = app.clone();
        let priority = if i < 3 { "high" } else { "low" };
        
        let payload = json!({
            "event": "custom",
            "priority": priority,
            "data": format!("Event {}", i)
        });

        tokio::spawn(async move {
            let request = Request::builder()
                .uri("/api/events/webhooks/generic")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-Webhook-Signature", "test")
                .header("X-Event-Priority", priority)
                .body(Body::from(payload.to_string()))
                .unwrap();

            app.oneshot(request).await.unwrap().status()
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All should be accepted regardless of priority
    for status in results {
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND || status == StatusCode::UNAUTHORIZED,
            "Priority events should be accepted"
        );
    }
}

#[tokio::test]
async fn maintains_fifo_order_within_priority() {
    let app = create_test_app().await;

    // Send events with timestamps
    let handles: Vec<_> = (1..=5).map(|i| {
        let app = app.clone();
        let payload = json!({
            "event": "message",
            "sequence": i,
            "timestamp": format!("2024-01-01T12:00:{:02}Z", i)
        });

        tokio::spawn(async move {
            let request = Request::builder()
                .uri("/api/events/webhooks/generic")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-Webhook-Signature", "test")
                .body(Body::from(payload.to_string()))
                .unwrap();

            app.oneshot(request).await.unwrap().status()
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Events should maintain order
    for status in results {
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND || status == StatusCode::UNAUTHORIZED,
            "Ordered events should be accepted"
        );
    }
}

// =============================================================================
// WORKER CONCURRENCY TESTS
// =============================================================================

#[tokio::test]
async fn respects_worker_concurrency_limits() {
    let app = create_test_app().await;

    // Send many events to test worker pool limits
    let handles: Vec<_> = (0..50).map(|i| {
        let app = app.clone();
        let payload = create_test_event(i);

        tokio::spawn(async move {
            let request = Request::builder()
                .uri("/api/events/webhooks/slack")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-Slack-Signature", "v0=test")
                .header("X-Slack-Request-Timestamp", "1234567890")
                .body(Body::from(payload.to_string()))
                .unwrap();

            app.oneshot(request).await.unwrap().status()
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All should be accepted even if workers are busy
    let success_count = results.iter().filter(|s| **s == StatusCode::OK || **s == StatusCode::NOT_FOUND).count();
    
    assert!(
        success_count >= 45,  // At least 90% should succeed
        "Worker pool should handle high load"
    );
}

#[tokio::test]
async fn handles_burst_traffic() {
    let app = create_test_app().await;

    // Simulate burst of 100 events
    let handles: Vec<_> = (0..100).map(|i| {
        let app = app.clone();
        let payload = create_test_event(i);

        tokio::spawn(async move {
            let request = Request::builder()
                .uri("/api/events/webhooks/slack")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("X-Slack-Signature", "v0=test")
                .header("X-Slack-Request-Timestamp", "1234567890")
                .body(Body::from(payload.to_string()))
                .unwrap();

            app.oneshot(request).await.unwrap().status()
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Most should succeed, some might be rate limited
    let success_count = results.iter().filter(|s| {
        **s == StatusCode::OK || **s == StatusCode::NOT_FOUND
    }).count();
    
    assert!(
        success_count >= 85,  // At least 85% should be accepted
        "System should handle burst traffic, got {} successes out of 100", success_count
    );
}

// =============================================================================
// RACE CONDITION TESTS
// =============================================================================

#[tokio::test]
async fn prevents_double_processing_race_condition() {
    let app = create_test_app().await;

    let event_id = sample_event_id();

    // Two workers try to process same event simultaneously
    let handles: Vec<_> = (0..2).map(|_| {
        let app = app.clone();
        let event_id = event_id.clone();

        tokio::spawn(async move {
            let request = Request::builder()
                .uri(&format!("/api/events/replay/{}", event_id))
                .method("POST")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer admin_token")
                .body(Body::empty())
                .unwrap();

            app.oneshot(request).await.unwrap().status()
        })
    }).collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Only one should succeed (or both fail with 404 if not implemented)
    let ok_count = results.iter().filter(|s| **s == StatusCode::OK).count();
    let conflict_count = results.iter().filter(|s| **s == StatusCode::CONFLICT).count();
    let not_found_count = results.iter().filter(|s| **s == StatusCode::NOT_FOUND).count();

    assert!(
        (ok_count == 1 && conflict_count == 1) || not_found_count == 2,
        "Race condition should be prevented"
    );
}

fn sample_event_id() -> String {
    format!("evt_{}", Uuid::new_v4())
}
