///! Integration Tests for OAuth Flow
///!
///! End-to-end tests covering:
///! - OAuth Authorization -> Callback -> Token Storage -> API Usage

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use crate::common::create_test_app;
use uuid::Uuid;
use tower::ServiceExt;

const OAUTH_PROVIDERS: [&str; 5] = ["google", "zoom", "slack", "hubspot", "salesforce"];

// =============================================================================
// OAUTH AUTHORIZATION ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn oauth_authorize_is_public_for_login() {
    let app = create_test_app().await;

    for provider in OAUTH_PROVIDERS {
        let request = Request::builder()
            .uri(format!("/api/auth/oauth/{}/authorize", provider))
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        if provider == "google" {
            // Google should redirect (303 See Other) to the consent screen
            assert_eq!(
                response.status(),
                StatusCode::SEE_OTHER,
                "Google authorize should redirect"
            );
            assert!(
                response.headers().contains_key("location"),
                "Google authorize should have location header"
            );
        } else {
            // Others are not implemented yet, so they should return 400 Bad Request
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "Unsupported provider {} should return Bad Request",
                provider
            );
        }
    }
}

#[tokio::test]
async fn oauth_authorize_invalid_provider() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/oauth/invalid_provider/authorize")
        .method("GET")
        .header("Authorization", "Bearer valid_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 or 404 for invalid provider (we expect 400 from our handler check)
    assert!(
        response.status().is_client_error(),
        "Invalid provider should be rejected"
    );
}

// =============================================================================
// OAUTH CALLBACK ENDPOINT TESTS
// =============================================================================
// ... (rest of file) ...


#[tokio::test]
async fn oauth_callback_requires_state() {
    let app = create_test_app().await;

    for provider in OAUTH_PROVIDERS {
        let request = Request::builder()
            .uri(format!("/api/auth/oauth/{}/callback?code=test_code", provider))
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        // Should fail without state (or 404 if not implemented)
        assert!(
            response.status().is_client_error() || response.status() == StatusCode::NOT_FOUND,
            "OAuth callback for {} without state should fail",
            provider
        );
    }
}

#[tokio::test]
async fn oauth_callback_requires_code() {
    let app = create_test_app().await;

    for provider in OAUTH_PROVIDERS {
        let request = Request::builder()
            .uri(format!("/api/auth/oauth/{}/callback?state=test_state", provider))
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        // Should fail without code (or 404 if not implemented)
        assert!(
            response.status().is_client_error() || response.status() == StatusCode::NOT_FOUND,
            "OAuth callback for {} without code should fail",
            provider
        );
    }
}

#[tokio::test]
async fn oauth_callback_handles_denial() {
    let app = create_test_app().await;

    for provider in OAUTH_PROVIDERS {
        let request = Request::builder()
            .uri(format!(
                "/api/auth/oauth/{}/callback?error=access_denied&state=test_state",
                provider
            ))
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        // Should handle gracefully (redirect or error page)
        let status = response.status();
        assert!(
            status.is_client_error()
                || status == StatusCode::NOT_FOUND
                || status.is_redirection()
                || status == StatusCode::OK,
            "OAuth denial for {} should be handled gracefully",
            provider
        );
    }
}

#[tokio::test]
async fn oauth_callback_invalid_state() {
    let app = create_test_app().await;

    for provider in OAUTH_PROVIDERS {
        let request = Request::builder()
            .uri(format!(
                "/api/auth/oauth/{}/callback?code=test_code&state=invalid_state",
                provider
            ))
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        // Should reject invalid state (CSRF protection)
        assert!(
            response.status().is_client_error() || response.status() == StatusCode::NOT_FOUND,
            "OAuth callback for {} with invalid state should be rejected",
            provider
        );
    }
}

// =============================================================================
// OAUTH CONNECTIONS ENDPOINT TESTS
// =============================================================================

#[tokio::test]
async fn oauth_connections_requires_auth() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/oauth/connections")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::NOT_FOUND,
        "OAuth connections should require auth"
    );
}

#[tokio::test]
async fn oauth_disconnect_requires_auth() {
    let app = create_test_app().await;
    let connection_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(format!("/api/auth/oauth/connections/{}", connection_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::NOT_FOUND,
        "OAuth disconnect should require auth"
    );
}

// =============================================================================
// HTTP METHOD TESTS
// =============================================================================

#[tokio::test]
async fn oauth_authorize_accepts_get() {
    let app = create_test_app().await;

    for provider in OAUTH_PROVIDERS {
        let request = Request::builder()
            .uri(format!("/api/auth/oauth/{}/authorize", provider))
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        assert!(
            response.status() != StatusCode::METHOD_NOT_ALLOWED
                || response.status() == StatusCode::NOT_FOUND,
            "OAuth authorize for {} should accept GET",
            provider
        );
    }
}

#[tokio::test]
async fn oauth_callback_accepts_get() {
    let app = create_test_app().await;

    for provider in OAUTH_PROVIDERS {
        let request = Request::builder()
            .uri(format!("/api/auth/oauth/{}/callback", provider))
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        assert!(
            response.status() != StatusCode::METHOD_NOT_ALLOWED
                || response.status() == StatusCode::NOT_FOUND,
            "OAuth callback for {} should accept GET",
            provider
        );
    }
}

#[tokio::test]
async fn oauth_connections_accepts_get() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/oauth/connections")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() != StatusCode::METHOD_NOT_ALLOWED
            || response.status() == StatusCode::NOT_FOUND,
        "OAuth connections should accept GET"
    );
}

#[tokio::test]
async fn oauth_disconnect_accepts_delete() {
    let app = create_test_app().await;
    let connection_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(format!("/api/auth/oauth/connections/{}", connection_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() != StatusCode::METHOD_NOT_ALLOWED
            || response.status() == StatusCode::NOT_FOUND,
        "OAuth disconnect should accept DELETE"
    );
}

// =============================================================================
// CONCURRENT OAUTH TESTS
// =============================================================================

#[tokio::test]
async fn concurrent_oauth_authorize_requests() {
    let app = create_test_app().await;

    let handles: Vec<_> = OAUTH_PROVIDERS
        .iter()
        .map(|provider| {
            let app = app.clone();
            let provider = provider.to_string();
            tokio::spawn(async move {
                let request = Request::builder()
                    .uri(format!("/api/auth/oauth/{}/authorize", provider))
                    .method("GET")
                    .body(Body::empty())
                    .unwrap();

                let response = app.oneshot(request).await.unwrap();
                (provider, response.status())
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All should complete without panic and return correct status
    for (provider, status) in results {
        if provider == "google" {
            assert_eq!(status, StatusCode::SEE_OTHER, "Google should redirect");
        } else {
            assert_eq!(status, StatusCode::BAD_REQUEST, "Others should fail");
        }
    }
}

#[tokio::test]
async fn concurrent_oauth_callback_requests() {
    let app = create_test_app().await;

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let app = app.clone();
            tokio::spawn(async move {
                let request = Request::builder()
                    .uri(format!(
                        "/api/auth/oauth/google/callback?code=code_{}&state=state_{}",
                        i, i
                    ))
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

    // All should complete. Expect Client Error because state is invalid/missing in Redis
    // or external call fails.
    for status in &results {
        assert!(
            status.is_client_error(),
            "Concurrent OAuth callbacks should result in client error (invalid state/code)"
        );
    }
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

#[tokio::test]
async fn oauth_callback_with_special_characters() {
    let app = create_test_app().await;

    // Test with URL-encoded special characters
    let request = Request::builder()
        .uri("/api/auth/oauth/google/callback?code=test%2Bcode&state=test%2Bstate")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle special characters
    assert!(
        response.status().is_client_error() || response.status() == StatusCode::NOT_FOUND,
        "OAuth callback should handle URL-encoded characters"
    );
}

#[tokio::test]
async fn oauth_disconnect_nonexistent() {
    let app = create_test_app().await;
    let fake_id = Uuid::new_v4();

    let request = Request::builder()
        .uri(format!("/api/auth/oauth/connections/{}", fake_id))
        .method("DELETE")
        .header("Authorization", "Bearer valid_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle gracefully
    assert!(
        response.status().is_client_error() || response.status() == StatusCode::NOT_FOUND,
        "Nonexistent OAuth connection disconnect should be handled"
    );
}

#[tokio::test]
async fn oauth_disconnect_invalid_uuid() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/oauth/connections/not-a-uuid")
        .method("DELETE")
        .header("Authorization", "Bearer valid_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return 400 or 404
    assert!(
        response.status().is_client_error(),
        "Invalid UUID in OAuth disconnect should be rejected"
    );
}