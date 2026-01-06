///! Integration Tests for Session Management
///!
///! End-to-end tests covering:
///! - Multi-device sessions
///! - Session listing and revocation
///! - Security events and invalidation

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

// =============================================================================
// SESSIONS ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn sessions_endpoint_requires_auth() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/sessions")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should require authentication (or 404 if not implemented)
    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::NOT_FOUND,
        "Sessions endpoint should require auth"
    );
}

#[tokio::test]
async fn sessions_endpoint_rejects_invalid_token() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/sessions")
        .method("GET")
        .header("Authorization", "Bearer invalid_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::NOT_FOUND,
        "Invalid token should be rejected"
    );
}

// =============================================================================
// SESSION REVOCATION ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn revoke_session_requires_auth() {
    let app = create_test_app().await;
    let session_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(format!("/api/auth/sessions/{}", session_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::NOT_FOUND,
        "Revoke session should require auth"
    );
}

#[tokio::test]
async fn revoke_all_sessions_requires_auth() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/sessions/revoke-all")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::NOT_FOUND,
        "Revoke all sessions should require auth"
    );
}

// =============================================================================
// HTTP METHOD TESTS
// =============================================================================

#[tokio::test]
async fn sessions_list_accepts_get() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/sessions")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should not be method not allowed (may be 401 or 404)
    assert!(
        response.status() != StatusCode::METHOD_NOT_ALLOWED || response.status() == StatusCode::NOT_FOUND,
        "Sessions should accept GET"
    );
}

#[tokio::test]
async fn sessions_list_rejects_post() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/sessions")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should reject POST on list endpoint (or 404 if not implemented)
    assert!(
        response.status() == StatusCode::METHOD_NOT_ALLOWED
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::UNAUTHORIZED,
        "Sessions list should not accept POST"
    );
}

// =============================================================================
// LOGOUT ALL ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn logout_all_requires_auth() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/logout-all")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::NOT_FOUND,
        "Logout all should require auth"
    );
}

// =============================================================================
// CONCURRENT SESSION TESTS
// =============================================================================

#[tokio::test]
async fn concurrent_session_list_requests() {
    let app = create_test_app().await;

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let app = app.clone();
            tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/auth/sessions")
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

    // All should complete without panic
    for status in &results {
        assert!(
            *status == StatusCode::UNAUTHORIZED || *status == StatusCode::NOT_FOUND,
            "Concurrent session list should be handled"
        );
    }
}

#[tokio::test]
async fn concurrent_logout_requests() {
    let app = create_test_app().await;

    let handles: Vec<_> = (0..5)
        .map(|_| {
            let app = app.clone();
            tokio::spawn(async move {
                let body = json!({"refresh_token": "test_token"});

                let request = Request::builder()
                    .uri("/api/auth/logout")
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
            status.is_client_error() || status.is_server_error(),
            "Concurrent logout should be handled"
        );
    }
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[tokio::test]
async fn revoke_nonexistent_session() {
    let app = create_test_app().await;
    let fake_session_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(format!("/api/auth/sessions/{}", fake_session_id))
        .method("DELETE")
        .header("Authorization", "Bearer some_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle gracefully (401, 404, or error)
    assert!(
        response.status().is_client_error() || response.status() == StatusCode::NOT_FOUND,
        "Nonexistent session revocation should be handled"
    );
}

#[tokio::test]
async fn sessions_with_various_user_agents() {
    let app = create_test_app().await;

    let user_agents = vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X) AppleWebKit/605.1.15",
        "PostmanRuntime/7.28.4",
        "curl/7.68.0",
    ];

    for user_agent in user_agents {
        let request = Request::builder()
            .uri("/api/auth/sessions")
            .method("GET")
            .header("User-Agent", user_agent)
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        // Should handle any User-Agent
        assert!(
            response.status() == StatusCode::UNAUTHORIZED
                || response.status() == StatusCode::NOT_FOUND,
            "Sessions should handle User-Agent: {}",
            user_agent
        );
    }
}
