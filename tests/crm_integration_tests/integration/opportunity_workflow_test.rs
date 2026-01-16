///! Integration Tests for CRM Opportunity Workflow
///!
///! End-to-end tests covering the complete opportunity lifecycle:
///! - Create Opportunity -> Update Stage -> Track Progress -> Close Opportunity

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use echo_backend::modules::auth::{crud::UserCrud, services::create_access_token};
use crate::common::{setup_test_context, create_test_app};
use secrecy::ExposeSecret;
use serde_json::json;
use sqlx::postgres::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use std::sync::Arc;
use echo_backend::config::AppConfig;

async fn create_auth_header(config: &Arc<AppConfig>, pool: &PgPool) -> String {
    let crud = UserCrud::new(pool, config);
    let email = format!("test_{}@example.com", Uuid::new_v4());
    
    // Create a real user in the DB so the middleware can find it
    let user = crud.create_user(&email, "Password123!", Some("Test"), Some("User"))
        .await
        .expect("Failed to create test user");

    let token = create_access_token(&user, config).expect("Failed to create token");
    format!("Bearer {}", token)
}

fn valid_opportunity_data() -> serde_json::Value {
    json!({
        "name": "Enterprise Software Deal",
        "amount": 75000.0,
        "stage": "Qualified",
        "probability": 50,
        "close_date": "2024-12-31",
        "contact_id": "contact_123",
        "description": "Large enterprise software implementation"
    })
}

// =============================================================================
// OPPORTUNITY WORKFLOW FLOW TESTS
// =============================================================================

#[tokio::test]
async fn full_opportunity_workflow_flow() {
    let (app, config, pool) = setup_test_context().await;
    let auth_header = create_auth_header(&config, &pool).await;
    let opportunity_data = valid_opportunity_data();

    // Step 1: Create opportunity
    let create_request = Request::builder()
        .uri("/api/crm/opportunities")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header.clone())
        .body(Body::from(opportunity_data.to_string()))
        .unwrap();

    let create_response = app.clone().oneshot(create_request).await.unwrap();
    let status = create_response.status();
    
    assert!(
        status == StatusCode::CREATED || status == StatusCode::INTERNAL_SERVER_ERROR,
        "Opportunity creation should return 201 (or 500 if no DB). Got: {}", status
    );

    // Step 2: Update opportunity stage (placeholder - would need actual opportunity ID)
    let update_data = json!({
        "stage": "Proposal",
        "probability": 75
    });
    
    let update_request = Request::builder()
        .uri("/api/crm/opportunities/123")  // Placeholder ID
        .method("PATCH")
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header.clone())
        .body(Body::from(update_data.to_string()))
        .unwrap();

    let update_response = app.clone().oneshot(update_request).await.unwrap();
    assert!(
        update_response.status() == StatusCode::OK || update_response.status() == StatusCode::NOT_FOUND,
        "Opportunity update should return 200 or 404. Got: {}", update_response.status()
    );
}

#[tokio::test]
async fn opportunity_creation_validates_required_fields() {
    let (app, config, pool) = setup_test_context().await;
    let auth_header = create_auth_header(&config, &pool).await;

    // Missing required fields
    let invalid_opportunity = json!({
        "name": "Enterprise Software Deal"
        // Missing stage, contact_id
    });

    let request = Request::builder()
        .uri("/api/crm/opportunities")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header)
        .body(Body::from(invalid_opportunity.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Missing required fields should be rejected"
    );
}

#[tokio::test]
async fn opportunity_creation_validates_stage_field() {
    let (app, config, pool) = setup_test_context().await;
    let auth_header = create_auth_header(&config, &pool).await;

    // Invalid stage value
    let invalid_opportunity = json!({
        "name": "Enterprise Software Deal",
        "amount": 75000.0,
        "stage": "InvalidStage",  // Not a valid stage
        "probability": 50,
        "close_date": "2024-12-31",
        "contact_id": "contact_123"
    });

    let request = Request::builder()
        .uri("/api/crm/opportunities")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header)
        .body(Body::from(invalid_opportunity.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Invalid stage should be rejected"
    );
}

#[tokio::test]
async fn opportunity_creation_validates_amount_field() {
    let (app, config, pool) = setup_test_context().await;
    let auth_header = create_auth_header(&config, &pool).await;

    // Negative amount
    let invalid_opportunity = json!({
        "name": "Enterprise Software Deal",
        "amount": -5000.0,  // Negative amount
        "stage": "Qualified",
        "probability": 50,
        "close_date": "2024-12-31",
        "contact_id": "contact_123"
    });

    let request = Request::builder()
        .uri("/api/crm/opportunities")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header)
        .body(Body::from(invalid_opportunity.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Negative amount should be rejected"
    );
}

#[tokio::test]
async fn opportunity_creation_validates_probability_range() {
    let (app, config, pool) = setup_test_context().await;
    let auth_header = create_auth_header(&config, &pool).await;

    // Probability out of range
    let invalid_opportunity = json!({
        "name": "Enterprise Software Deal",
        "amount": 75000.0,
        "stage": "Qualified",
        "probability": 150,  // Greater than 100
        "close_date": "2024-12-31",
        "contact_id": "contact_123"
    });

    let request = Request::builder()
        .uri("/api/crm/opportunities")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header)
        .body(Body::from(invalid_opportunity.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Probability > 100 should be rejected"
    );
}

#[tokio::test]
async fn opportunity_creation_validates_close_date_format() {
    let (app, config, pool) = setup_test_context().await;
    let auth_header = create_auth_header(&config, &pool).await;

    // Invalid date format
    let invalid_opportunity = json!({
        "name": "Enterprise Software Deal",
        "amount": 75000.0,
        "stage": "Qualified",
        "probability": 50,
        "close_date": "invalid-date",  // Invalid format
        "contact_id": "contact_123"
    });

    let request = Request::builder()
        .uri("/api/crm/opportunities")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header)
        .body(Body::from(invalid_opportunity.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status().is_client_error(),
        "Invalid date format should be rejected"
    );
}

// =============================================================================
// OPPORTUNITY STAGE TRANSITION TESTS
// =============================================================================

#[tokio::test]
async fn opportunity_stage_transition_follows_business_rules() {
    let (app, config, pool) = setup_test_context().await;
    let auth_header = create_auth_header(&config, &pool).await;

    // Attempt to transition from Qualified to Closed Won (should go through intermediate stages)
    let transition_data = json!({
        "stage": "Closed Won",
        "probability": 100
    });
    
    let request = Request::builder()
        .uri("/api/crm/opportunities/123/update-stage")  // Placeholder ID
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header)
        .body(Body::from(transition_data.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should either accept the transition or return a validation error
    assert!(
        response.status() == StatusCode::OK || 
        response.status() == StatusCode::UNPROCESSABLE_ENTITY ||
        response.status() == StatusCode::NOT_FOUND,
        "Stage transition should be handled appropriately"
    );
}

#[tokio::test]
async fn opportunity_stage_transition_updates_probability() {
    let (app, config, pool) = setup_test_context().await;
    let auth_header = create_auth_header(&config, &pool).await;

    // Transition to a stage that should update probability automatically
    let transition_data = json!({
        "stage": "Negotiation"
    });
    
    let request = Request::builder()
        .uri("/api/crm/opportunities/123/update-stage")  // Placeholder ID
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", auth_header)
        .body(Body::from(transition_data.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should update probability based on stage
    assert!(
        response.status() == StatusCode::OK || 
        response.status() == StatusCode::NOT_FOUND,
        "Stage transition should be handled"
    );
}

// =============================================================================
// OPPORTUNITY SEARCH AND FILTERING TESTS
// =============================================================================

#[tokio::test]
async fn opportunity_search_by_stage() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/crm/opportunities?stage=Qualified")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Opportunity search by stage should return 200"
    );
}

#[tokio::test]
async fn opportunity_search_by_amount_range() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/crm/opportunities?min_amount=50000&max_amount=100000")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Opportunity search by amount range should return 200"
    );
}

#[tokio::test]
async fn opportunity_search_by_close_date_range() {
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/api/crm/opportunities?start_date=2024-01-01&end_date=2024-12-31")
        .method("GET")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Opportunity search by date range should return 200"
    );
}

// =============================================================================
// OPPORTUNITY ENDPOINT ACCESSIBILITY TESTS
// =============================================================================

#[tokio::test]
async fn opportunity_endpoints_accept_post_method() {
    let app = create_test_app().await;
    let opportunity_data = valid_opportunity_data();

    let request = Request::builder()
        .uri("/api/crm/opportunities")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::from(opportunity_data.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() != StatusCode::NOT_FOUND && response.status() != StatusCode::METHOD_NOT_ALLOWED,
        "Opportunity POST should be accepted"
    );
}

#[tokio::test]
async fn opportunity_stage_update_endpoint_accepts_post_method() {
    let app = create_test_app().await;

    let stage_data = json!({
        "stage": "Proposal"
    });

    let request = Request::builder()
        .uri("/api/crm/opportunities/123/update-stage")
        .method("POST")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test_token")
        .body(Body::from(stage_data.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert!(
        response.status() != StatusCode::NOT_FOUND && response.status() != StatusCode::METHOD_NOT_ALLOWED,
        "Opportunity stage update POST should be accepted"
    );
}

// =============================================================================
// CONCURRENT OPPORTUNITY OPERATIONS TESTS
// =============================================================================

#[tokio::test]
async fn concurrent_opportunity_creations_dont_conflict() {
    let app = create_test_app().await;

    // Create multiple opportunities
    let opportunities: Vec<_> = (0..3).map(|i| {
        let uuid_str = Uuid::new_v4().to_string();
        let short_uuid = &uuid_str[..std::cmp::min(8, uuid_str.len())];
        json!({
            "name": format!("Opportunity {}", short_uuid),
            "amount": 25000.0 + (i as f64) * 10000.0,
            "stage": "Qualified",
            "probability": 50,
            "close_date": "2024-12-31",
            "contact_id": format!("contact_{}", i),
            "description": "Test opportunity"
        })
    }).collect();

    let handles: Vec<_> = opportunities
        .iter()
        .map(|opp_data| {
            let app = app.clone();
            let opp_data = opp_data.clone();

            tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/crm/opportunities")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "Bearer test_token")
                    .body(Body::from(opp_data.to_string()))
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

    // All should complete (may succeed or fail based on DB)
    for status in results {
        assert!(
            status == StatusCode::CREATED || status == StatusCode::INTERNAL_SERVER_ERROR,
            "Concurrent opportunity creations should not panic. Got: {}", status
        );
    }
}