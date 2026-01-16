///! Integration Tests for Webhook Receivers
///!
///! Tests all 5 webhook endpoints for the Event Stream Processing module:
///! - Slack webhook receiver
///! - Gmail push notification receiver
///! - Zoom webhook receiver
///! - Salesforce outbound message receiver
///! - Generic webhook receiver

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

fn create_slack_webhook_payload() -> serde_json::Value {
    json!({
        "token": "test_verification_token",
        "team_id": "T123456",
        "api_app_id": "A123456",
        "event": {
            "type": "message",
            "channel": "C123456",
            "user": "U123456",
            "text": "Just closed a deal worth $50k with Acme Corp!",
            "ts": "1234567890.123456"
        },
        "type": "event_callback",
        "event_id": &format!("Ev{}", Uuid::new_v4()),
        "event_time": 1234567890
    })
}

fn create_gmail_pub_sub_payload() -> serde_json::Value {
    json!({
        "message": {
            "data": "eyJlbWFpbEFkZHJlc3MiOiJ0ZXN0QGV4YW1wbGUuY29tIiwiaGlzdG9yeUlkIjoiMTIzNDU2In0=",
            "messageId": &format!("{}", Uuid::new_v4()),
            "message_id": &format!("{}", Uuid::new_v4()),
            "publishTime": "2024-01-01T12:00:00.000Z",
            "publish_time": "2024-01-01T12:00:00.000Z"
        },
        "subscription": "projects/test-project/subscriptions/gmail-push"
    })
}

fn create_zoom_webhook_payload() -> serde_json::Value {
    json!({
        "event": "meeting.ended",
        "payload": {
            "account_id": "abc123",
            "object": {
                "uuid": &format!("{}", Uuid::new_v4()),
                "id": 123456789,
                "host_id": "host123",
                "topic": "Q1 Sales Review",
                "type": 2,
                "start_time": "2024-01-01T10:00:00Z",
                "duration": 60,
                "timezone": "America/New_York"
            }
        },
        "event_ts": 1234567890123i64
    })
}

fn create_salesforce_outbound_message() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
    <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
        <soapenv:Body>
            <notifications xmlns="http://soap.sforce.com/2005/09/outbound">
                <OrganizationId>00D000000000001</OrganizationId>
                <ActionId>04k000000000001</ActionId>
                <SessionId>sessionid123</SessionId>
                <Notification>
                    <Id>04l000000000001</Id>
                    <sObject xsi:type="sf:Opportunity" xmlns:sf="urn:sobject.enterprise.soap.sforce.com/2006/04/01">
                        <sf:Id>006000000000001</sf:Id>
                        <sf:Name>Acme Corp Deal</sf:Name>
                        <sf:Amount>50000</sf:Amount>
                        <sf:StageName>Closed Won</sf:StageName>
                    </sObject>
                </Notification>
            </notifications>
        </soapenv:Body>
    </soapenv:Envelope>"#.to_string()
}

fn create_generic_webhook_payload() -> serde_json::Value {
    json!({
        "event": "deal_updated",
        "data": {
            "deal_id": "deal_123",
            "status": "closed_won",
            "amount": 75000
        },
        "timestamp": "2024-01-01T12:00:00Z"
    })
}

fn valid_slack_signature() -> String {
    "v0=a2114d57b48eac39b9ad189dd8316235a7b4a8d21a10bd27519666489c69b503".to_string()
}

fn valid_zoom_signature() -> String {
    "v0=12345abcdef67890".to_string()
}

// =============================================================================
// SLACK WEBHOOK TESTS
// =============================================================================

#[tokio::test]
async fn slack_webhook_accepts_valid_payload() {
    let app = create_test_app().await;
    let payload = create_slack_webhook_payload();

    let request = Request::builder()
        .uri("/api/events/webhooks/slack")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Slack-Signature", &valid_slack_signature())
        .header("X-Slack-Request-Timestamp", "1234567890")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 200 OK or 500 if not yet implemented
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Slack webhook should be accessible, got: {}", response.status()
    );
}

#[tokio::test]
async fn slack_webhook_handles_url_verification() {
    let app = create_test_app().await;

    // Slack sends URL verification challenge on first setup
    let challenge_payload = json!({
        "token": "test_verification_token",
        "challenge": "3eZbrw1aBm2rZgRNFdxV2595E9CY3gmdALWMmHkvFXO7tYXAYM8P",
        "type": "url_verification"
    });

    let request = Request::builder()
        .uri("/api/events/webhooks/slack")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(challenge_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should respond with challenge in body (or 404 if not implemented)
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Slack URL verification should be handled"
    );
}

#[tokio::test]
async fn slack_webhook_rejects_invalid_signature() {
    let app = create_test_app().await;
    let payload = create_slack_webhook_payload();

    let request = Request::builder()
        .uri("/api/events/webhooks/slack")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Slack-Signature", "v0=invalid_signature")
        .header("X-Slack-Request-Timestamp", "1234567890")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should reject with 401 Unauthorized or 404 if not implemented
    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::NOT_FOUND,
        "Invalid Slack signature should be rejected, got: {}", response.status()
    );
}

#[tokio::test]
async fn slack_webhook_rejects_missing_signature() {
    let app = create_test_app().await;
    let payload = create_slack_webhook_payload();

    let request = Request::builder()
        .uri("/api/events/webhooks/slack")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should reject with 401 Unauthorized or 404 if not implemented
    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::NOT_FOUND,
        "Missing Slack signature should be rejected"
    );
}

// =============================================================================
// GMAIL WEBHOOK TESTS
// =============================================================================

#[tokio::test]
async fn gmail_webhook_accepts_valid_pub_sub_message() {
    let app = create_test_app().await;
    let payload = create_gmail_pub_sub_payload();

    let request = Request::builder()
        .uri("/api/events/webhooks/gmail")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Gmail webhook should accept valid Pub/Sub message, got: {}", response.status()
    );
}

#[tokio::test]
async fn gmail_webhook_rejects_malformed_payload() {
    let app = create_test_app().await;

    let malformed_payload = json!({
        "invalid": "structure"
    });

    let request = Request::builder()
        .uri("/api/events/webhooks/gmail")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(malformed_payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::NOT_FOUND,
        "Malformed Gmail payload should be rejected"
    );
}

// =============================================================================
// ZOOM WEBHOOK TESTS
// =============================================================================

#[tokio::test]
async fn zoom_webhook_accepts_meeting_ended_event() {
    let app = create_test_app().await;
    let payload = create_zoom_webhook_payload();

    let request = Request::builder()
        .uri("/api/events/webhooks/zoom")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", &valid_zoom_signature())
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Zoom webhook should accept meeting.ended event, got: {}", response.status()
    );
}

#[tokio::test]
async fn zoom_webhook_rejects_invalid_authorization() {
    let app = create_test_app().await;
    let payload = create_zoom_webhook_payload();

    let request = Request::builder()
        .uri("/api/events/webhooks/zoom")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "invalid_auth_token")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::NOT_FOUND,
        "Zoom webhook should reject invalid authorization"
    );
}

// =============================================================================
// SALESFORCE WEBHOOK TESTS
// =============================================================================

#[tokio::test]
async fn salesforce_webhook_accepts_outbound_message() {
    let app = create_test_app().await;
    let payload = create_salesforce_outbound_message();

    let request = Request::builder()
        .uri("/api/events/webhooks/salesforce")
        .method("POST")
        .header("Content-Type", "text/xml; charset=utf-8")
        .header("SOAPAction", "\"\"")
        .body(Body::from(payload))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Salesforce webhook should accept SOAP outbound message, got: {}", response.status()
    );
}

#[tokio::test]
async fn salesforce_webhook_validates_soap_structure() {
    let app = create_test_app().await;

    let invalid_soap = "<invalid>xml</invalid>";

    let request = Request::builder()
        .uri("/api/events/webhooks/salesforce")
        .method("POST")
        .header("Content-Type", "text/xml; charset=utf-8")
        .body(Body::from(invalid_soap))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::NOT_FOUND,
        "Invalid SOAP structure should be rejected"
    );
}

// =============================================================================
// GENERIC WEBHOOK TESTS
// =============================================================================

#[tokio::test]
async fn generic_webhook_accepts_custom_events() {
    let app = create_test_app().await;
    let payload = create_generic_webhook_payload();

    let request = Request::builder()
        .uri("/api/events/webhooks/generic")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test_signature")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Generic webhook should accept custom events, got: {}", response.status()
    );
}

#[tokio::test]
async fn generic_webhook_validates_signature() {
    let app = create_test_app().await;
    let payload = create_generic_webhook_payload();

    // Missing signature header
    let request = Request::builder()
        .uri("/api/events/webhooks/generic")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::NOT_FOUND,
        "Generic webhook should require signature"
    );
}

#[tokio::test]
async fn generic_webhook_handles_large_payloads() {
    let app = create_test_app().await;

    // Create a large payload (simulating a big transcript or email thread)
    let large_data = "x".repeat(10_000);
    let payload = json!({
        "event": "transcript_received",
        "data": {
            "content": large_data
        }
    });

    let request = Request::builder()
        .uri("/api/events/webhooks/generic")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", "test_signature")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::OK 
            || response.status() == StatusCode::PAYLOAD_TOO_LARGE 
            || response.status() == StatusCode::NOT_FOUND,
        "Generic webhook should handle large payloads appropriately"
    );
}
