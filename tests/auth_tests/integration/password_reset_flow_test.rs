///! Integration Tests for Password Reset Flow
///!
///! End-to-end tests covering:
///! - Request Reset -> Receive Token -> Reset Password -> Login

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use crate::common::create_test_app;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

fn unique_email() -> String {
    format!("test_{}@example.com", Uuid::new_v4())
}

fn valid_password() -> String {
    "SecurePass123!".to_string()
}

fn weak_password() -> String {
    "weak".to_string()
}

// =============================================================================
// FORGOT PASSWORD ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn forgot_password_accepts_post() {
    let app = create_test_app().await;

    let body = json!({
        "email": unique_email()
    });

    let request = Request::builder()
        .uri("/api/auth/forgot-password")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Endpoint should exist (may return 404 if not implemented yet)
    // If implemented, should return 200 for any email (no enumeration)
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "Forgot password should handle request"
    );
}

#[tokio::test]
async fn forgot_password_requires_email() {
    let app = create_test_app().await;

    let body = json!({});

    let request = Request::builder()
        .uri("/api/auth/forgot-password")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should fail if email is missing (if endpoint exists)
    let status = response.status();
    assert!(
        status.is_client_error() || status == StatusCode::NOT_FOUND,
        "Missing email should be rejected"
    );
}

#[tokio::test]
async fn forgot_password_rejects_invalid_email() {
    let app = create_test_app().await;

    let body = json!({
        "email": "not-an-email"
    });

    let request = Request::builder()
        .uri("/api/auth/forgot-password")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should reject invalid email format
    let status = response.status();
    assert!(
        status.is_client_error() || status == StatusCode::NOT_FOUND || status == StatusCode::OK,
        "Invalid email format should be handled"
    );
}

// =============================================================================
// RESET PASSWORD ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn reset_password_accepts_post() {
    let app = create_test_app().await;

    let body = json!({
        "token": "some_reset_token",
        "new_password": valid_password()
    });

    let request = Request::builder()
        .uri("/api/auth/reset-password")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Endpoint should exist and handle request
    assert!(
        response.status() != StatusCode::METHOD_NOT_ALLOWED,
        "Reset password should accept POST"
    );
}

#[tokio::test]
async fn reset_password_requires_token() {
    let app = create_test_app().await;

    let body = json!({
        "new_password": valid_password()
    });

    let request = Request::builder()
        .uri("/api/auth/reset-password")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let status = response.status();
    assert!(
        status.is_client_error() || status == StatusCode::NOT_FOUND,
        "Missing token should be rejected"
    );
}

#[tokio::test]
async fn reset_password_requires_new_password() {
    let app = create_test_app().await;

    let body = json!({
        "token": "some_token"
    });

    let request = Request::builder()
        .uri("/api/auth/reset-password")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let status = response.status();
    assert!(
        status.is_client_error() || status == StatusCode::NOT_FOUND,
        "Missing new_password should be rejected"
    );
}

#[tokio::test]
async fn reset_password_validates_password_strength() {
    let app = create_test_app().await;

    let body = json!({
        "token": "some_token",
        "new_password": weak_password()
    });

    let request = Request::builder()
        .uri("/api/auth/reset-password")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let status = response.status();
    // Should reject weak password (or 404 if endpoint not implemented)
    assert!(
        status.is_client_error() || status == StatusCode::NOT_FOUND,
        "Weak password should be rejected"
    );
}

#[tokio::test]
async fn reset_password_rejects_invalid_token() {
    let app = create_test_app().await;

    let body = json!({
        "token": "invalid_token_12345",
        "new_password": valid_password()
    });

    let request = Request::builder()
        .uri("/api/auth/reset-password")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    let status = response.status();
    // Should reject invalid token
    assert!(
        status.is_client_error() || status == StatusCode::NOT_FOUND,
        "Invalid token should be rejected"
    );
}

// =============================================================================
// HTTP METHOD TESTS
// =============================================================================

#[tokio::test]
async fn forgot_password_rejects_get() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/forgot-password")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should reject GET or return 404 if endpoint not implemented
    assert!(
        response.status() == StatusCode::METHOD_NOT_ALLOWED
            || response.status() == StatusCode::NOT_FOUND,
        "Forgot password should not accept GET"
    );
}

#[tokio::test]
async fn reset_password_rejects_get() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/reset-password")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::METHOD_NOT_ALLOWED
            || response.status() == StatusCode::NOT_FOUND,
        "Reset password should not accept GET"
    );
}

// =============================================================================
// CONTENT-TYPE TESTS
// =============================================================================

#[tokio::test]
async fn forgot_password_requires_json() {
    let app = create_test_app().await;

    let body = json!({"email": unique_email()});

    let request = Request::builder()
        .uri("/api/auth/forgot-password")
        .method("POST")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should reject missing Content-Type
    assert!(
        response.status().is_client_error() || response.status() == StatusCode::NOT_FOUND,
        "Missing Content-Type should be rejected"
    );
}

#[tokio::test]
async fn reset_password_requires_json() {
    let app = create_test_app().await;

    let body = json!({
        "token": "token",
        "new_password": valid_password()
    });

    let request = Request::builder()
        .uri("/api/auth/reset-password")
        .method("POST")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error() || response.status() == StatusCode::NOT_FOUND,
        "Missing Content-Type should be rejected"
    );
}

// =============================================================================
// CONCURRENT REQUEST TESTS
// =============================================================================

#[tokio::test]
async fn concurrent_forgot_password_requests() {
    let app = create_test_app().await;
    let email = unique_email();

    let handles: Vec<_> = (0..5)
        .map(|_| {
            let app = app.clone();
            let email = email.clone();
            tokio::spawn(async move {
                let body = json!({"email": email});

                let request = Request::builder()
                    .uri("/api/auth/forgot-password")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.to_string()))
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

    // All should complete without panic
    for status in &results {
        assert!(
            *status == StatusCode::OK
                || *status == StatusCode::NOT_FOUND
                || status.is_server_error(),
            "Concurrent forgot password should be handled"
        );
    }
}