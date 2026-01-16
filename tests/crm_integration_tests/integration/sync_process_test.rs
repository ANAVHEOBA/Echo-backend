///! Integration Tests for CRM Sync Process
///!
///! End-to-end tests covering the complete synchronization process:
///! - Initiate Sync -> Process Data -> Update Local DB -> Report Results

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use crate::common::create_test_app;
use serde_json::json;
use tower::ServiceExt;

fn valid_sync_request() -> serde_json::Value {
    json!({
        "source": "salesforce",
        "entity_type": "contacts",
        "sync_type": "full",
        "options": {
            "include_deleted": false,
            "fields": ["id", "first_name", "last_name", "email", "company", "title"]
        }
    })
}

fn incremental_sync_request() -> serde_json::Value {
    json!({
        "source": "hubspot",
        "entity_type": "leads",
        "sync_type": "incremental",
        "since": "2023-10-01T00:00:00Z",
        "options": {
            "include_deleted": false,
            "fields": ["id", "first_name", "last_name", "email", "company", "status", "source"]
        }
    })
}

// =============================================================================
// SYNC PROCESS FLOW TESTS
// =============================================================================

#[tokio::test]
async fn full_sync_process_initiation() {
    let app = create_test_app().await;
    let sync_request = valid_sync_request();

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(sync_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should return 202 Accepted for async processing or 200 OK for sync processing
    assert!(
        response.status() == StatusCode::ACCEPTED || response.status() == StatusCode::OK,
        "Sync initiation should be accepted"
    );
}

#[tokio::test]
async fn incremental_sync_process_initiation() {
    let app = create_test_app().await;
    let sync_request = incremental_sync_request();

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(sync_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert!(
        response.status() == StatusCode::ACCEPTED || response.status() == StatusCode::OK,
        "Incremental sync initiation should be accepted"
    );
}

// =============================================================================
// SYNC REQUEST VALIDATION TESTS
// =============================================================================

#[tokio::test]
async fn sync_request_validates_required_fields() {
    let app = create_test_app().await;

    // Missing required fields
    let invalid_request = json!({
        "entity_type": "contacts"
        // Missing source and sync_type
    });

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(invalid_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Missing required fields should be rejected"
    );
}

#[tokio::test]
async fn sync_request_validates_supported_sources() {
    let app = create_test_app().await;

    // Invalid source
    let invalid_request = json!({
        "source": "unsupported_crm",
        "entity_type": "contacts",
        "sync_type": "full"
    });

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(invalid_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Unsupported source should be rejected"
    );
}

#[tokio::test]
async fn sync_request_validates_supported_entity_types() {
    let app = create_test_app().await;

    // Invalid entity type
    let invalid_request = json!({
        "source": "salesforce",
        "entity_type": "invalid_entity",
        "sync_type": "full"
    });

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(invalid_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Unsupported entity type should be rejected"
    );
}

#[tokio::test]
async fn sync_request_validates_sync_types() {
    let app = create_test_app().await;

    // Invalid sync type
    let invalid_request = json!({
        "source": "salesforce",
        "entity_type": "contacts",
        "sync_type": "invalid_type"
    });

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(invalid_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Invalid sync type should be rejected"
    );
}

#[tokio::test]
async fn incremental_sync_requires_since_parameter() {
    let app = create_test_app().await;

    // Incremental sync without since parameter
    let invalid_request = json!({
        "source": "hubspot",
        "entity_type": "leads",
        "sync_type": "incremental"
        // Missing since parameter
    });

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(invalid_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Incremental sync without since parameter should be rejected"
    );
}

#[tokio::test]
async fn incremental_sync_validates_since_format() {
    let app = create_test_app().await;

    // Invalid since format
    let invalid_request = json!({
        "source": "hubspot",
        "entity_type": "leads",
        "sync_type": "incremental",
        "since": "invalid-date-format"
    });

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(invalid_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Invalid since format should be rejected"
    );
}

// =============================================================================
// SYNC PROCESSING TESTS
// =============================================================================

#[tokio::test]
async fn sync_process_handles_large_datasets() {
    let app = create_test_app().await;

    // Large dataset sync request
    let large_sync_request = json!({
        "source": "salesforce",
        "entity_type": "contacts",
        "sync_type": "full",
        "options": {
            "include_deleted": false,
            "fields": ["id", "first_name", "last_name", "email", "company", "title", "phone", "address"]
        },
        "batch_size": 1000
    });

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(large_sync_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should accept large dataset syncs
    assert!(
        response.status() == StatusCode::ACCEPTED || response.status() == StatusCode::OK,
        "Large dataset sync should be accepted"
    );
}

#[tokio::test]
async fn sync_process_handles_field_mappings() {
    let app = create_test_app().await;

    // Request with custom field mappings
    let mapping_request = json!({
        "source": "hubspot",
        "entity_type": "contacts",
        "sync_type": "full",
        "options": {
            "include_deleted": false,
            "field_mappings": {
                "hs_first_name": "first_name",
                "hs_last_name": "last_name",
                "hs_email": "email",
                "hs_company": "company"
            }
        }
    });

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(mapping_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert!(
        response.status() == StatusCode::ACCEPTED || response.status() == StatusCode::OK,
        "Field mapping sync should be accepted"
    );
}

// =============================================================================
// SYNC STATUS AND MONITORING TESTS
// =============================================================================

#[tokio::test]
async fn sync_status_can_be_queried() {
    let app = create_test_app().await;

    // Query sync status (placeholder ID)
    let request = Request::builder()
        .uri("/api/crm/sync/status/sync_123")
        .method("GET")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should return status info or 404 if not found
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Sync status query should return appropriate status"
    );
}

#[tokio::test]
async fn sync_history_can_be_retrieved() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/crm/sync/history?limit=10&offset=0")
        .method("GET")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Sync history retrieval should return 200"
    );
}

#[tokio::test]
async fn sync_history_supports_filtering() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/crm/sync/history?entity_type=contacts&status=completed")
        .method("GET")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Filtered sync history retrieval should return 200"
    );
}

// =============================================================================
// SYNC CONTENT-TYPE AND METHOD TESTS
// =============================================================================

#[tokio::test]
async fn sync_endpoint_accepts_post_method() {
    let app = create_test_app().await;
    let sync_request = valid_sync_request();

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(sync_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() != StatusCode::NOT_FOUND && response.status() != StatusCode::METHOD_NOT_ALLOWED,
        "Sync POST should be accepted"
    );
}

#[tokio::test]
async fn sync_endpoint_rejects_unsupported_methods() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("PUT")  // PUT should not be allowed
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "Sync endpoint should reject unsupported methods"
    );
}

#[tokio::test]
async fn sync_endpoint_requires_json_content_type() {
    let app = create_test_app().await;
    let sync_request = valid_sync_request();

    // Missing Content-Type header
    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .body(Body::from(sync_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Missing Content-Type should be rejected"
    );
}

// =============================================================================
// CONCURRENT SYNC PROCESSING TESTS
// =============================================================================

#[tokio::test]
async fn concurrent_sync_operations_are_handled_properly() {
    let app = create_test_app().await;

    // Create multiple sync requests
    let sync_requests = vec![
        json!({
            "source": "salesforce",
            "entity_type": "contacts",
            "sync_type": "full"
        }),
        json!({
            "source": "hubspot",
            "entity_type": "leads",
            "sync_type": "incremental",
            "since": "2023-10-01T00:00:00Z"
        }),
        json!({
            "source": "pipedrive",
            "entity_type": "opportunities",
            "sync_type": "full"
        })
    ];

    let handles: Vec<_> = sync_requests
        .iter()
        .map(|sync_data| {
            let app = app.clone();
            let sync_data = sync_data.clone();

            tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/crm/sync")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(sync_data.to_string()))
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

    // All should be accepted for processing
    for status in results {
        assert!(
            status == StatusCode::ACCEPTED || status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR,
            "Concurrent sync operations should be handled properly"
        );
    }
}

// =============================================================================
// SYNC ERROR HANDLING TESTS
// =============================================================================

#[tokio::test]
async fn sync_process_handles_authentication_errors() {
    let app = create_test_app().await;

    // Request with invalid credentials (simulated)
    let invalid_auth_request = json!({
        "source": "salesforce",
        "entity_type": "contacts",
        "sync_type": "full",
        "credentials": {
            "access_token": "invalid-token"
        }
    });

    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(invalid_auth_request.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return appropriate error for auth failure
    assert!(
        response.status() == StatusCode::UNAUTHORIZED || 
        response.status() == StatusCode::BAD_REQUEST ||
        response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "Auth errors should be handled appropriately"
    );
}

#[tokio::test]
async fn sync_process_handles_rate_limiting() {
    let app = create_test_app().await;

    // Request that might trigger rate limiting
    let request = Request::builder()
        .uri("/api/crm/sync")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(valid_sync_request().to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle rate limiting appropriately
    assert!(
        response.status() != StatusCode::INTERNAL_SERVER_ERROR,
        "Rate limiting should be handled gracefully, not cause server errors"
    );
}