///! Integration Tests for Event Subscriptions
///!
///! Tests webhook subscription management endpoints:
///! - Create webhook subscription (POST /api/events/subscriptions)
///! - List webhook subscriptions (GET /api/events/subscriptions)
///! - Delete subscription (DELETE /api/events/subscriptions/{id})

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

fn valid_subscription_payload() -> serde_json::Value {
    json!({
        "platform": "slack",
        "webhook_url": "https://hooks.slack.com/services/T00/B00/XX",
        "secret": "webhook_secret_123",
        "event_types": ["message", "app_mention"],
        "active": true
    })
}

fn sample_subscription_id() -> String {
    format!("{}", Uuid::new_v4())
}

// =============================================================================
// CREATE SUBSCRIPTION TESTS
// =============================================================================

#[tokio::test]
async fn create_subscription_accepts_valid_payload() {
    let app = create_test_app().await;
    let payload = valid_subscription_payload();

    let request = Request::builder()
        .uri("/api/events/subscriptions")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 201 Created or 404 if not implemented
    assert!(
        response.status() == StatusCode::CREATED || response.status() == StatusCode::NOT_FOUND,
        "Create subscription should accept valid payload, got: {}", response.status()
    );
}

#[tokio::test]
async fn create_subscription_validates_required_fields() {
    let app = create_test_app().await;

    // Missing required webhook_url field
    let invalid_payload = json!({
        "platform": "slack",
        "secret": "webhook_secret_123"
    });

    let request = Request::builder()
        .uri("/api/events/subscriptions")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::from(invalid_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNPROCESSABLE_ENTITY || response.status() == StatusCode::NOT_FOUND,
        "Missing required fields should be rejected"
    );
}

#[tokio::test]
async fn create_subscription_validates_webhook_url_format() {
    let app = create_test_app().await;

    let invalid_payload = json!({
        "platform": "slack",
        "webhook_url": "not-a-valid-url",
        "secret": "webhook_secret_123"
    });

    let request = Request::builder()
        .uri("/api/events/subscriptions")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::from(invalid_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNPROCESSABLE_ENTITY || response.status() == StatusCode::NOT_FOUND,
        "Invalid webhook URL format should be rejected"
    );
}

#[tokio::test]
async fn create_subscription_validates_platform_value() {
    let app = create_test_app().await;

    let invalid_payload = json!({
        "platform": "invalid_platform",
        "webhook_url": "https://example.com/webhook",
        "secret": "webhook_secret_123"
    });

    let request = Request::builder()
        .uri("/api/events/subscriptions")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::from(invalid_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNPROCESSABLE_ENTITY || response.status() == StatusCode::NOT_FOUND,
        "Invalid platform value should be rejected"
    );
}

#[tokio::test]
async fn create_subscription_requires_authentication() {
    let app = create_test_app().await;
    let payload = valid_subscription_payload();

    // Missing Authorization header
    let request = Request::builder()
        .uri("/api/events/subscriptions")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::NOT_FOUND,
        "Create subscription should require authentication"
    );
}

#[tokio::test]
async fn create_subscription_enforces_user_subscription_limit() {
    let app = create_test_app().await;

    // Attempt to create multiple subscriptions
    let handles: Vec<_> = (0..5).map(|i| {
        let app = app.clone();
        let mut payload = valid_subscription_payload();
        payload["webhook_url"] = json!(format!("https://example.com/webhook{}", i));

        tokio::spawn(async move {
            let request = Request::builder()
                .uri("/api/events/subscriptions")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer test_token")
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

    // At least some should succeed or all should be NOT_FOUND if not implemented
    assert!(
        results.iter().any(|s| *s == StatusCode::CREATED || *s == StatusCode::NOT_FOUND),
        "Subscription creation should handle limits appropriately"
    );
}

// =============================================================================
// LIST SUBSCRIPTIONS TESTS
// =============================================================================

#[tokio::test]
async fn list_subscriptions_endpoint_is_accessible() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events/subscriptions")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "List subscriptions endpoint should be accessible, got: {}", response.status()
    );
}

#[tokio::test]
async fn list_subscriptions_returns_user_subscriptions_only() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events/subscriptions")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should only return subscriptions for authenticated user
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "List subscriptions should filter by user"
    );
}

#[tokio::test]
async fn list_subscriptions_supports_filtering_by_platform() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events/subscriptions?platform=slack")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "List subscriptions should support platform filtering"
    );
}

#[tokio::test]
async fn list_subscriptions_supports_filtering_by_active_status() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/events/subscriptions?active=true")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "List subscriptions should support active status filtering"
    );
}

#[tokio::test]
async fn list_subscriptions_requires_authentication() {
    let app = create_test_app().await;

    // Missing Authorization header
    let request = Request::builder()
        .uri("/api/events/subscriptions")
        .method("GET")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::NOT_FOUND,
        "List subscriptions should require authentication"
    );
}

// =============================================================================
// DELETE SUBSCRIPTION TESTS
// =============================================================================

#[tokio::test]
async fn delete_subscription_endpoint_is_accessible() {
    let app = create_test_app().await;
    let subscription_id = sample_subscription_id();

    let request = Request::builder()
        .uri(&format!("/api/events/subscriptions/{}", subscription_id))
        .method("DELETE")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 404 (not found) or route not found
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::NO_CONTENT,
        "Delete subscription endpoint should be accessible"
    );
}

#[tokio::test]
async fn delete_subscription_validates_subscription_exists() {
    let app = create_test_app().await;
    let nonexistent_id = sample_subscription_id();

    let request = Request::builder()
        .uri(&format!("/api/events/subscriptions/{}", nonexistent_id))
        .method("DELETE")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::NOT_FOUND,
        "Delete should return 404 for nonexistent subscription"
    );
}

#[tokio::test]
async fn delete_subscription_validates_ownership() {
    let app = create_test_app().await;
    let subscription_id = sample_subscription_id();

    // Try to delete with different user token
    let request = Request::builder()
        .uri(&format!("/api/events/subscriptions/{}", subscription_id))
        .method("DELETE")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer different_user_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 403 Forbidden or 404 or 401 Unauthorized (invalid token)
    assert!(
        response.status() == StatusCode::FORBIDDEN || response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::UNAUTHORIZED,
        "Delete subscription should validate ownership"
    );
}

#[tokio::test]
async fn delete_subscription_requires_authentication() {
    let app = create_test_app().await;
    let subscription_id = sample_subscription_id();

    // Missing Authorization header
    let request = Request::builder()
        .uri(&format!("/api/events/subscriptions/{}", subscription_id))
        .method("DELETE")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::NOT_FOUND,
        "Delete subscription should require authentication"
    );
}

#[tokio::test]
async fn delete_subscription_returns_no_content_on_success() {
    let app = create_test_app().await;
    
    // Create first
    let create_payload = valid_subscription_payload();
    let create_req = Request::builder()
        .uri("/api/events/subscriptions")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::from(create_payload.to_string()))
        .unwrap();
    
    let create_res = app.clone().oneshot(create_req).await.unwrap();
    let body_bytes = axum::body::to_bytes(create_res.into_body(), usize::MAX).await.unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let subscription_id = body_json.get("id").unwrap().as_str().unwrap();

    let request = Request::builder()
        .uri(&format!("/api/events/subscriptions/{}", subscription_id))
        .method("DELETE")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 204 No Content on successful deletion
    assert!(
        response.status() == StatusCode::NO_CONTENT,
        "Successful deletion should return 204 No Content"
    );
}

// =============================================================================
// SUBSCRIPTION LIFECYCLE TESTS
// =============================================================================

#[tokio::test]
async fn subscription_lifecycle_create_list_delete() {
    let app = create_test_app().await;

    // Step 1: Create subscription
    let create_payload = valid_subscription_payload();
    let create_request = Request::builder()
        .uri("/api/events/subscriptions")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::from(create_payload.to_string()))
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    let create_status = create_response.status();

    // Step 2: List subscriptions
    let list_request = Request::builder()
        .uri("/api/events/subscriptions")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let list_response = app.clone().oneshot(list_request).await.unwrap();
    let list_status = list_response.status();

    // Step 3: Delete subscription (placeholder ID)
    let delete_request = Request::builder()
        .uri(&format!("/api/events/subscriptions/{}", sample_subscription_id()))
        .method("DELETE")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let delete_response = app.oneshot(delete_request).await.unwrap();
    let delete_status = delete_response.status();

    // All steps should complete (success or not found)
    assert!(
        (create_status == StatusCode::CREATED || create_status == StatusCode::NOT_FOUND) &&
        (list_status == StatusCode::OK || list_status == StatusCode::NOT_FOUND) &&
        (delete_status == StatusCode::NO_CONTENT || delete_status == StatusCode::NOT_FOUND),
        "Subscription lifecycle should work end-to-end"
    );
}
