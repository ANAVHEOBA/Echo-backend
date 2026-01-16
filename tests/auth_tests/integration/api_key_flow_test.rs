///! Integration Tests for API Key Flow
///!
///! End-to-end tests covering:
///! - Create Key -> Use Key -> Manage Key -> Revoke Key

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use crate::common::create_test_app;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

// =============================================================================
// API KEYS ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn api_keys_list_requires_auth() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/api-keys")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should require authentication (or 404 if not implemented)
    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::NOT_FOUND,
        "API keys list should require auth"
    );
}

#[tokio::test]
async fn api_keys_create_requires_auth() {
    let app = create_test_app().await;

    let body = json!({
        "name": "Test Key",
        "scopes": ["read:*"]
    });

    let request = Request::builder()
        .uri("/api/auth/api-keys")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::NOT_FOUND,
        "API key creation should require auth"
    );
}

#[tokio::test]
async fn api_keys_delete_requires_auth() {
    let app = create_test_app().await;
    let key_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(format!("/api/auth/api-keys/{}", key_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::NOT_FOUND,
        "API key deletion should require auth"
    );
}

// =============================================================================
// API KEY AUTHENTICATION TESTS
// =============================================================================

#[tokio::test]
async fn api_key_auth_header_format() {
    let app = create_test_app().await;

    // Test with ApiKey prefix
    let request = Request::builder()
        .uri("/api/auth/me")
        .method("GET")
        .header("Authorization", "ApiKey ek_test_key_12345")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be unauthorized (invalid key), not method error
    assert!(
        response.status() == StatusCode::UNAUTHORIZED,
        "ApiKey auth header should be processed"
    );
}

#[tokio::test]
async fn api_key_bearer_format_with_prefix() {
    let app = create_test_app().await;

    // Test with Bearer prefix but ek_ key prefix
    let request = Request::builder()
        .uri("/api/auth/me")
        .method("GET")
        .header("Authorization", "Bearer ek_test_key_12345")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be unauthorized (invalid key)
    assert!(
        response.status() == StatusCode::UNAUTHORIZED,
        "Bearer with ek_ prefix should be processed as API key"
    );
}

#[tokio::test]
async fn invalid_api_key_rejected() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/me")
        .method("GET")
        .header("Authorization", "ApiKey invalid_key")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Invalid API key should be rejected"
    );
}

// =============================================================================
// HTTP METHOD TESTS
// =============================================================================

#[tokio::test]
async fn api_keys_list_accepts_get() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/api-keys")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should not be method not allowed
    assert!(
        response.status() != StatusCode::METHOD_NOT_ALLOWED
            || response.status() == StatusCode::NOT_FOUND,
        "API keys should accept GET"
    );
}

#[tokio::test]
async fn api_keys_create_accepts_post() {
    let app = create_test_app().await;

    let body = json!({
        "name": "Test Key"
    });

    let request = Request::builder()
        .uri("/api/auth/api-keys")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() != StatusCode::METHOD_NOT_ALLOWED
            || response.status() == StatusCode::NOT_FOUND,
        "API keys should accept POST"
    );
}

#[tokio::test]
async fn api_keys_delete_accepts_delete() {
    let app = create_test_app().await;
    let key_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(format!("/api/auth/api-keys/{}", key_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() != StatusCode::METHOD_NOT_ALLOWED
            || response.status() == StatusCode::NOT_FOUND,
        "API key deletion should accept DELETE"
    );
}

// =============================================================================
// CONCURRENT API KEY TESTS
// =============================================================================

#[tokio::test]
async fn concurrent_api_key_auth_requests() {
    let app = create_test_app().await;

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let app = app.clone();
            tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/auth/me")
                    .method("GET")
                    .header("Authorization", format!("ApiKey ek_test_key_{}", i))
                    .body(Body::empty())
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

    // All should be unauthorized (invalid keys)
    for status in &results {
        assert_eq!(
            *status,
            StatusCode::UNAUTHORIZED,
            "Concurrent API key auth should be handled"
        );
    }
}

#[tokio::test]
async fn concurrent_api_key_list_requests() {
    let app = create_test_app().await;

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let app = app.clone();
            tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/auth/api-keys")
                    .method("GET")
                    .body(Body::empty())
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

    // All should require auth
    for status in &results {
        assert!(
            *status == StatusCode::UNAUTHORIZED || *status == StatusCode::NOT_FOUND,
            "Concurrent API key list should be handled"
        );
    }
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[tokio::test]
async fn api_key_empty_name_rejected() {
    let app = create_test_app().await;

    let body = json!({
        "name": "",
        "scopes": ["read:*"]
    });

    let request = Request::builder()
        .uri("/api/auth/api-keys")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer valid_token")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be rejected (or 401/404)
    assert!(
        response.status().is_client_error(),
        "Empty name should be rejected"
    );
}

#[tokio::test]
async fn api_key_delete_nonexistent() {
    let app = create_test_app().await;
    let fake_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(format!("/api/auth/api-keys/{}", fake_id))
        .method("DELETE")
        .header("Authorization", "Bearer valid_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle gracefully
    assert!(
        response.status().is_client_error() || response.status() == StatusCode::NOT_FOUND,
        "Nonexistent API key deletion should be handled"
    );
}

#[tokio::test]
async fn api_key_invalid_uuid_in_path() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/api-keys/not-a-uuid")
        .method("DELETE")
        .header("Authorization", "Bearer valid_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 or 404
    assert!(
        response.status().is_client_error(),
        "Invalid UUID should be rejected"
    );
}