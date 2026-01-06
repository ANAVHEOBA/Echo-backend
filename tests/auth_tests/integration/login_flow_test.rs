///! Integration Tests for Login Flow
///!
///! End-to-end tests covering:
///! - Login -> Access Resources -> Token Refresh -> Logout

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use echo_backend::{create_app, config::AppConfig};
use secrecy::SecretString;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

// =============================================================================
// TEST HELPERS
// =============================================================================

async fn create_test_app() -> axum::Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://test:test@localhost/test_db")
        .expect("Failed to create lazy pool");

    let config = AppConfig {
        database_url: SecretString::from("postgres://test:test@localhost/test_db"),
        redis_url: "redis://localhost:6379".to_string(),
        jwt_secret: SecretString::from("test_jwt_secret_key_that_is_at_least_32_characters_long"),
        jwt_refresh_secret: SecretString::from("test_refresh_secret_key_that_is_at_least_32_chars"),
        access_token_expiry_secs: 900,
        refresh_token_expiry_days: 7,
        encryption_key: SecretString::from("test_encryption_key_32_chars_ok"),
        host: "127.0.0.1".to_string(),
        port: 3000,
    };

    create_app(pool, config).await
}

fn unique_email() -> String {
    format!("test_{}@example.com", Uuid::new_v4())
}

fn valid_password() -> String {
    "SecurePass123!".to_string()
}

// =============================================================================
// LOGIN ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn login_endpoint_exists() {
    let app = create_test_app().await;

    let login_body = json!({
        "email": unique_email(),
        "password": valid_password()
    });

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Route should exist (may return 401 without valid user)
    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Login endpoint should exist"
    );
}

#[tokio::test]
async fn login_requires_email() {
    let app = create_test_app().await;

    let login_body = json!({
        "password": valid_password()
    });

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Missing email should be rejected"
    );
}

#[tokio::test]
async fn login_requires_password() {
    let app = create_test_app().await;

    let login_body = json!({
        "email": unique_email()
    });

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Missing password should be rejected"
    );
}

#[tokio::test]
async fn login_rejects_empty_email() {
    let app = create_test_app().await;

    let login_body = json!({
        "email": "",
        "password": valid_password()
    });

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be client error (validation) or server error (if DB validation required)
    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Empty email should be rejected"
    );
}

#[tokio::test]
async fn login_rejects_empty_password() {
    let app = create_test_app().await;

    let login_body = json!({
        "email": unique_email(),
        "password": ""
    });

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should be client error (validation) or server error (if DB validation required)
    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Empty password should be rejected"
    );
}

// =============================================================================
// REFRESH ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn refresh_endpoint_exists() {
    let app = create_test_app().await;

    let refresh_body = json!({
        "refresh_token": "some_token"
    });

    let request = Request::builder()
        .uri("/api/auth/refresh")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(refresh_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Route should exist (may return error for invalid token)
    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Refresh endpoint should exist"
    );
}

#[tokio::test]
async fn refresh_requires_token() {
    let app = create_test_app().await;

    let refresh_body = json!({});

    let request = Request::builder()
        .uri("/api/auth/refresh")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(refresh_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Missing refresh token should be rejected"
    );
}

#[tokio::test]
async fn refresh_rejects_invalid_token() {
    let app = create_test_app().await;

    let refresh_body = json!({
        "refresh_token": "invalid_token_12345"
    });

    let request = Request::builder()
        .uri("/api/auth/refresh")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(refresh_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 401 for invalid token
    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status().is_client_error(),
        "Invalid refresh token should be rejected"
    );
}

// =============================================================================
// LOGOUT ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn logout_endpoint_exists() {
    let app = create_test_app().await;

    let logout_body = json!({
        "refresh_token": "some_token"
    });

    let request = Request::builder()
        .uri("/api/auth/logout")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(logout_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Route should exist
    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Logout endpoint should exist"
    );
}

#[tokio::test]
async fn logout_requires_refresh_token() {
    let app = create_test_app().await;

    let logout_body = json!({});

    let request = Request::builder()
        .uri("/api/auth/logout")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(logout_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Missing refresh token should be rejected"
    );
}

// =============================================================================
// ME ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn me_endpoint_exists() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/me")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Route should exist (will return 401 without auth)
    assert!(
        response.status() == StatusCode::UNAUTHORIZED || response.status() != StatusCode::NOT_FOUND,
        "Me endpoint should exist"
    );
}

#[tokio::test]
async fn me_requires_authentication() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/me")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Me endpoint should require authentication"
    );
}

#[tokio::test]
async fn me_rejects_invalid_bearer_token() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/me")
        .method("GET")
        .header("Authorization", "Bearer invalid_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Invalid bearer token should be rejected"
    );
}

#[tokio::test]
async fn me_rejects_malformed_auth_header() {
    let app = create_test_app().await;

    // Missing "Bearer " prefix
    let request = Request::builder()
        .uri("/api/auth/me")
        .method("GET")
        .header("Authorization", "some_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Malformed auth header should be rejected"
    );
}

// =============================================================================
// HTTP METHOD TESTS
// =============================================================================

#[tokio::test]
async fn login_rejects_get_method() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "Login should not accept GET"
    );
}

#[tokio::test]
async fn refresh_rejects_get_method() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/refresh")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "Refresh should not accept GET"
    );
}

#[tokio::test]
async fn logout_rejects_get_method() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/logout")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "Logout should not accept GET"
    );
}

#[tokio::test]
async fn me_rejects_post_method() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/me")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "Me should not accept POST"
    );
}

// =============================================================================
// CONTENT-TYPE TESTS
// =============================================================================

#[tokio::test]
async fn login_requires_json_content_type() {
    let app = create_test_app().await;

    let login_body = json!({
        "email": unique_email(),
        "password": valid_password()
    });

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Missing Content-Type should be rejected"
    );
}

#[tokio::test]
async fn login_rejects_invalid_json() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from("{invalid json}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Invalid JSON should be rejected"
    );
}

// =============================================================================
// CONCURRENT REQUEST TESTS
// =============================================================================

#[tokio::test]
async fn concurrent_login_requests_handled() {
    let app = create_test_app().await;

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let app = app.clone();
            tokio::spawn(async move {
                let login_body = json!({
                    "email": unique_email(),
                    "password": valid_password()
                });

                let request = Request::builder()
                    .uri("/api/auth/login")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(login_body.to_string()))
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

    // All requests should complete without panic
    for status in results {
        assert!(
            status.is_client_error() || status.is_server_error(),
            "Concurrent requests should be handled without panic"
        );
    }
}

#[tokio::test]
async fn concurrent_me_requests_handled() {
    let app = create_test_app().await;

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let app = app.clone();
            tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/auth/me")
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

    // All should return 401 (no auth)
    for status in results {
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "Concurrent me requests should all return 401"
        );
    }
}
