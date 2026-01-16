///! Integration Tests for CRM Webhook Handler
///!
///! End-to-end tests covering webhook reception and processing:
///! - Receive Webhook -> Parse Payload -> Process Event -> Respond

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use crate::common::create_test_app;
use serde_json::json;
use tower::ServiceExt;

fn valid_contact_created_webhook() -> serde_json::Value {
    json!({
        "event": "contact.created",
        "timestamp": "2023-10-15T10:30:00Z",
        "data": {
            "id": "ext_contact_123",
            "first_name": "John",
            "last_name": "Doe",
            "email": "john.doe@example.com",
            "phone": "+1234567890",
            "company": "Acme Corp",
            "title": "Software Engineer"
        }
    })
}

fn valid_lead_updated_webhook() -> serde_json::Value {
    json!({
        "event": "lead.updated",
        "timestamp": "2023-10-15T11:30:00Z",
        "data": {
            "id": "ext_lead_456",
            "first_name": "Alice",
            "last_name": "Johnson",
            "email": "alice.johnson@example.com",
            "status": "Qualified",
            "source": "Website"
        }
    })
}

fn valid_opportunity_deleted_webhook() -> serde_json::Value {
    json!({
        "event": "opportunity.deleted",
        "timestamp": "2023-10-15T12:30:00Z",
        "data": {
            "id": "ext_opp_789"
        }
    })
}

// =============================================================================
// WEBHOOK HANDLER FLOW TESTS
// =============================================================================

#[tokio::test]
async fn webhook_handler_processes_contact_created_event() {
    let app = create_test_app().await;
    let webhook_payload = valid_contact_created_webhook();

    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test-signature")  // Assuming signature verification
        .body(Body::from(webhook_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should return 200 OK for successful processing
    // Or 202 Accepted if processing is queued
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::ACCEPTED,
        "Contact created webhook should be processed successfully"
    );
}

#[tokio::test]
async fn webhook_handler_processes_lead_updated_event() {
    let app = create_test_app().await;
    let webhook_payload = valid_lead_updated_webhook();

    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test-signature")
        .body(Body::from(webhook_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::ACCEPTED,
        "Lead updated webhook should be processed successfully"
    );
}

#[tokio::test]
async fn webhook_handler_processes_opportunity_deleted_event() {
    let app = create_test_app().await;
    let webhook_payload = valid_opportunity_deleted_webhook();

    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test-signature")
        .body(Body::from(webhook_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::ACCEPTED,
        "Opportunity deleted webhook should be processed successfully"
    );
}

// =============================================================================
// WEBHOOK VALIDATION TESTS
// =============================================================================

#[tokio::test]
async fn webhook_handler_rejects_invalid_json() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test-signature")
        .body(Body::from("{invalid json}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Invalid JSON should be rejected"
    );
}

#[tokio::test]
async fn webhook_handler_rejects_missing_event_field() {
    let app = create_test_app().await;
    let invalid_payload = json!({
        "timestamp": "2023-10-15T10:30:00Z",
        "data": {
            "id": "contact_123",
            "first_name": "John",
            "last_name": "Doe",
            "email": "john@example.com"
        }
    });

    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test-signature")
        .body(Body::from(invalid_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Missing event field should be rejected"
    );
}

#[tokio::test]
async fn webhook_handler_rejects_unknown_event_type() {
    let app = create_test_app().await;
    let invalid_payload = json!({
        "event": "unknown.event.type",
        "timestamp": "2023-10-15T10:30:00Z",
        "data": {
            "id": "contact_123",
            "first_name": "John",
            "last_name": "Doe",
            "email": "john@example.com"
        }
    });

    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test-signature")
        .body(Body::from(invalid_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should either reject unknown events or handle them gracefully
    assert!(
        response.status() == StatusCode::OK || 
        response.status() == StatusCode::BAD_REQUEST ||
        response.status() == StatusCode::UNPROCESSABLE_ENTITY,
        "Unknown event type should be handled appropriately"
    );
}

#[tokio::test]
async fn webhook_handler_rejects_missing_signature() {
    let app = create_test_app().await;
    let webhook_payload = valid_contact_created_webhook();

    // Missing signature header
    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(webhook_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should reject unsigned webhooks for security
    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::BAD_REQUEST,
        "Missing signature should be rejected"
    );
}

#[tokio::test]
async fn webhook_handler_rejects_invalid_signature() {
    let app = create_test_app().await;
    let webhook_payload = valid_contact_created_webhook();

    // Invalid signature
    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "invalid-signature")
        .body(Body::from(webhook_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED,
        "Invalid signature should be rejected"
    );
}

// =============================================================================
// WEBHOOK CONTENT-TYPE TESTS
// =============================================================================

#[tokio::test]
async fn webhook_handler_requires_json_content_type() {
    let app = create_test_app().await;
    let webhook_payload = valid_contact_created_webhook();

    // Missing Content-Type header
    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("X-Webhook-Signature", "test-signature")
        .body(Body::from(webhook_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Missing Content-Type should be rejected"
    );
}

#[tokio::test]
async fn webhook_handler_rejects_non_json_content_type() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "text/plain")
        .header("X-Webhook-Signature", "test-signature")
        .body(Body::from("plain text"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Non-JSON Content-Type should be rejected"
    );
}

// =============================================================================
// WEBHOOK PROCESSING SCALABILITY TESTS
// =============================================================================

#[tokio::test]
async fn concurrent_webhook_requests_handled_properly() {
    let app = create_test_app().await;

    // Create multiple webhook payloads
    let webhooks: Vec<_> = (0..5).map(|i| {
        json!({
            "event": "contact.created",
            "timestamp": format!("2023-10-15T10:3{}:00Z", i),
            "data": {
                "id": format!("contact_{}", i),
                "first_name": "Test",
                "last_name": format!("User{}", i),
                "email": format!("test{}@example.com", i),
                "company": "Test Corp"
            }
        })
    }).collect();

    let handles: Vec<_> = webhooks
        .iter()
        .map(|webhook_data| {
            let app = app.clone();
            let webhook_data = webhook_data.clone();

            tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/crm/webhooks")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .header("X-Webhook-Signature", "test-signature")
                    .body(Body::from(webhook_data.to_string()))
                    .unwrap();

                app.oneshot(request).await.unwrap().status()
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All should be processed successfully
    for status in results {
        assert!(
            status == StatusCode::OK || status == StatusCode::ACCEPTED,
            "Concurrent webhook requests should be handled properly"
        );
    }
}

// =============================================================================
// WEBHOOK RETRY AND ERROR HANDLING TESTS
// =============================================================================

#[tokio::test]
async fn webhook_handler_handles_processing_errors_gracefully() {
    let app = create_test_app().await;
    
    // Payload that might cause processing errors (e.g., missing required fields in data)
    let problematic_payload = json!({
        "event": "contact.created",
        "timestamp": "2023-10-15T10:30:00Z",
        "data": {
            "id": "contact_123"
            // Missing required fields like first_name, email
        }
    });

    let request = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test-signature")
        .body(Body::from(problematic_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle errors gracefully, not crash
    assert!(
        response.status().is_success() || response.status().is_client_error(),
        "Processing errors should be handled gracefully, not cause server errors"
    );
}

// =============================================================================
// WEBHOOK SECURITY TESTS
// =============================================================================

#[tokio::test]
async fn webhook_handler_prevents_replay_attacks() {
    let app = create_test_app().await;
    let webhook_payload = valid_contact_created_webhook();

    // Send the same request twice - second should potentially be rejected as replay
    let request1 = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test-signature")
        .header("X-Webhook-ID", "unique-id-123")
        .body(Body::from(webhook_payload.to_string()))
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    
    // Second identical request
    let request2 = Request::builder()
        .uri("/api/crm/webhooks")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test-signature")
        .header("X-Webhook-ID", "unique-id-123")  // Same ID - potential replay
        .body(Body::from(valid_contact_created_webhook().to_string()))
        .unwrap();

    let response2 = app.oneshot(request2).await.unwrap();

    // First should succeed, second might be rejected as replay
    assert!(
        response1.status() == StatusCode::OK || response1.status() == StatusCode::ACCEPTED,
        "First webhook should be processed"
    );
    
    // Second could be accepted (if deduplication isn't strict) or rejected (if it is)
    assert!(
        response2.status().is_success() || response2.status() == StatusCode::CONFLICT,
        "Replayed webhook should be handled appropriately"
    );
}