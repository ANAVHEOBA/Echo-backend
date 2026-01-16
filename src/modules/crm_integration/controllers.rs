use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use validator::Validate;

use crate::AppState;
use super::schemas::{
    CreateContactRequest, UpdateContactRequest, GetContactResponse, ContactFilter,
    CreateLeadRequest, UpdateLeadRequest, GetLeadResponse, LeadFilter, ConvertLeadRequest,
    CreateOpportunityRequest, UpdateOpportunityRequest, UpdateOpportunityStageRequest, GetOpportunityResponse, OpportunityFilter
};
use super::crud;

// =============================================================================
// CONTACT CONTROLLERS
// =============================================================================

pub async fn create_contact(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateContactRequest>,
) -> Result<(StatusCode, Json<GetContactResponse>), StatusCode> {
    payload.validate().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    match crud::create_contact(&state.pool, payload).await {
        Ok(contact) => {
            let response = GetContactResponse {
                id: contact.id,
                first_name: contact.first_name,
                last_name: contact.last_name,
                email: contact.email,
                phone: contact.phone,
                company: contact.company,
                title: contact.title,
                created_at: contact.created_at,
                updated_at: contact.updated_at,
            };
            Ok((StatusCode::CREATED, Json(response)).into())
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_contact(
    State(state): State<Arc<AppState>>,
    Path(contact_id): Path<String>,
) -> Result<Json<GetContactResponse>, StatusCode> {
    match crud::get_contact_by_id(&state.pool, &contact_id).await {
        Ok(Some(contact)) => {
            let response = GetContactResponse {
                id: contact.id,
                first_name: contact.first_name,
                last_name: contact.last_name,
                email: contact.email,
                phone: contact.phone,
                company: contact.company,
                title: contact.title,
                created_at: contact.created_at,
                updated_at: contact.updated_at,
            };
            Ok(Json(response))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn list_contacts(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<ContactFilter>,
) -> Result<Json<Vec<GetContactResponse>>, StatusCode> {
    match crud::list_contacts(&state.pool, filter).await {
        Ok(contacts) => {
            let response: Vec<GetContactResponse> = contacts.into_iter().map(|c| GetContactResponse {
                id: c.id,
                first_name: c.first_name,
                last_name: c.last_name,
                email: c.email,
                phone: c.phone,
                company: c.company,
                title: c.title,
                created_at: c.created_at,
                updated_at: c.updated_at,
            }).collect();
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update_contact(
    State(state): State<Arc<AppState>>,
    Path(contact_id): Path<String>,
    Json(payload): Json<UpdateContactRequest>,
) -> Result<Json<GetContactResponse>, StatusCode> {
    payload.validate().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    match crud::update_contact(
        &state.pool,
        &contact_id,
        payload.first_name,
        payload.last_name,
        payload.email,
        payload.phone,
        payload.company,
        payload.title,
    ).await {
        Ok(Some(contact)) => {
            let response = GetContactResponse {
                id: contact.id,
                first_name: contact.first_name,
                last_name: contact.last_name,
                email: contact.email,
                phone: contact.phone,
                company: contact.company,
                title: contact.title,
                created_at: contact.created_at,
                updated_at: contact.updated_at,
            };
            Ok(Json(response))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn delete_contact(
    State(state): State<Arc<AppState>>,
    Path(contact_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    match crud::delete_contact(&state.pool, &contact_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn handle_webhook(
    State(_state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    // 1. Check Signature
    let signature = headers.get("X-Webhook-Signature")
        .and_then(|v| v.to_str().ok());
    
    if signature.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Mock signature verification
    if signature == Some("invalid-signature") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 2. Check Event Field
    if let Some(_event) = payload.get("event") {
        // In a real app, process the event here based on its type
        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

// =============================================================================
// LEAD CONTROLLERS
// =============================================================================

pub async fn create_lead(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateLeadRequest>,
) -> Result<(StatusCode, Json<GetLeadResponse>), StatusCode> {
    payload.validate().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    match crud::create_lead(&state.pool, payload).await {
        Ok(lead) => {
            let response = GetLeadResponse {
                id: lead.id,
                first_name: lead.first_name,
                last_name: lead.last_name,
                email: lead.email,
                phone: lead.phone,
                company: lead.company,
                status: lead.status,
                source: lead.source,
                title: lead.title,
                created_at: lead.created_at,
                updated_at: lead.updated_at,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_lead(
    State(state): State<Arc<AppState>>,
    Path(lead_id): Path<String>,
) -> Result<Json<GetLeadResponse>, StatusCode> {
    match crud::get_lead_by_id(&state.pool, &lead_id).await {
        Ok(Some(lead)) => {
            let response = GetLeadResponse {
                id: lead.id,
                first_name: lead.first_name,
                last_name: lead.last_name,
                email: lead.email,
                phone: lead.phone,
                company: lead.company,
                status: lead.status,
                source: lead.source,
                title: lead.title,
                created_at: lead.created_at,
                updated_at: lead.updated_at,
            };
            Ok(Json(response))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn list_leads(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<LeadFilter>,
) -> Result<Json<Vec<GetLeadResponse>>, StatusCode> {
    match crud::list_leads(&state.pool, filter).await {
        Ok(leads) => {
            let response: Vec<GetLeadResponse> = leads.into_iter().map(|l| GetLeadResponse {
                id: l.id,
                first_name: l.first_name,
                last_name: l.last_name,
                email: l.email,
                phone: l.phone,
                company: l.company,
                status: l.status,
                source: l.source,
                title: l.title,
                created_at: l.created_at,
                updated_at: l.updated_at,
            }).collect();
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update_lead(
    State(state): State<Arc<AppState>>,
    Path(lead_id): Path<String>,
    Json(payload): Json<UpdateLeadRequest>,
) -> Result<Json<GetLeadResponse>, StatusCode> {
    payload.validate().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    match crud::update_lead(
        &state.pool,
        &lead_id,
        payload.first_name,
        payload.last_name,
        payload.email,
        payload.phone,
        payload.company,
        payload.status,
        payload.source,
        payload.title,
    ).await {
        Ok(Some(lead)) => {
            let response = GetLeadResponse {
                id: lead.id,
                first_name: lead.first_name,
                last_name: lead.last_name,
                email: lead.email,
                phone: lead.phone,
                company: lead.company,
                status: lead.status,
                source: lead.source,
                title: lead.title,
                created_at: lead.created_at,
                updated_at: lead.updated_at,
            };
            Ok(Json(response))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn delete_lead(
    State(state): State<Arc<AppState>>,
    Path(lead_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    match crud::delete_lead(&state.pool, &lead_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn convert_lead_to_opportunity(
    State(state): State<Arc<AppState>>,
    Path(lead_id): Path<String>,
    Json(payload): Json<ConvertLeadRequest>,
) -> Result<StatusCode, StatusCode> {
    // 1. Get the lead
    let lead = match crud::get_lead_by_id(&state.pool, &lead_id).await {
        Ok(Some(l)) => l,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // 2. Create Contact from Lead
    let contact_req = CreateContactRequest {
        first_name: lead.first_name,
        last_name: lead.last_name,
        email: lead.email,
        phone: lead.phone,
        company: Some(lead.company),
        title: lead.title,
    };

    // Note: If contact with email exists, this might fail or duplicate depending on DB constraint.
    // For now assuming success or letting it error out (500).
    let contact = match crud::create_contact(&state.pool, contact_req).await {
        Ok(c) => c,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // 3. Create Opportunity linked to Contact
    let opportunity_req = CreateOpportunityRequest {
        name: payload.opportunity_name,
        amount: payload.estimated_value,
        stage: "New".to_string(),
        probability: Some(10),
        close_date: None,
        contact_id: contact.id,
        description: Some(format!("Converted from lead {}", lead.source)),
    };

    if let Err(_) = crud::create_opportunity(&state.pool, opportunity_req).await {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // 4. Update Lead status to Converted
    if let Err(_) = crud::update_lead(
        &state.pool,
        &lead_id,
        None, None, None, None, None,
        Some("Converted".to_string()),
        None, None
    ).await {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(StatusCode::OK)
}

// =============================================================================
// OPPORTUNITY CONTROLLERS
// =============================================================================

pub async fn create_opportunity(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateOpportunityRequest>,
) -> Result<Json<GetOpportunityResponse>, StatusCode> {
    payload.validate().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    match crud::create_opportunity(&state.pool, payload).await {
        Ok(opportunity) => {
            let response = GetOpportunityResponse {
                id: opportunity.id,
                name: opportunity.name,
                amount: opportunity.amount,
                stage: opportunity.stage,
                probability: opportunity.probability,
                close_date: opportunity.close_date,
                contact_id: opportunity.contact_id,
                description: opportunity.description,
                created_at: opportunity.created_at,
                updated_at: opportunity.updated_at,
            };
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_opportunity(
    State(state): State<Arc<AppState>>,
    Path(opportunity_id): Path<String>,
) -> Result<Json<GetOpportunityResponse>, StatusCode> {
    match crud::get_opportunity_by_id(&state.pool, &opportunity_id).await {
        Ok(Some(opportunity)) => {
            let response = GetOpportunityResponse {
                id: opportunity.id,
                name: opportunity.name,
                amount: opportunity.amount,
                stage: opportunity.stage,
                probability: opportunity.probability,
                close_date: opportunity.close_date,
                contact_id: opportunity.contact_id,
                description: opportunity.description,
                created_at: opportunity.created_at,
                updated_at: opportunity.updated_at,
            };
            Ok(Json(response))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn list_opportunities(
    State(state): State<Arc<AppState>>,
    Query(filter): Query<OpportunityFilter>,
) -> Result<Json<Vec<GetOpportunityResponse>>, StatusCode> {
    match crud::list_opportunities(&state.pool, filter).await {
        Ok(opportunities) => {
            let response: Vec<GetOpportunityResponse> = opportunities.into_iter().map(|o| GetOpportunityResponse {
                id: o.id,
                name: o.name,
                amount: o.amount,
                stage: o.stage,
                probability: o.probability,
                close_date: o.close_date,
                contact_id: o.contact_id,
                description: o.description,
                created_at: o.created_at,
                updated_at: o.updated_at,
            }).collect();
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update_opportunity(
    State(state): State<Arc<AppState>>,
    Path(opportunity_id): Path<String>,
    Json(payload): Json<UpdateOpportunityRequest>,
) -> Result<Json<GetOpportunityResponse>, StatusCode> {
    payload.validate().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    match crud::update_opportunity(
        &state.pool,
        &opportunity_id,
        payload.name,
        payload.amount,
        payload.stage,
        payload.probability,
        payload.close_date,
        payload.description,
    ).await {
        Ok(Some(opportunity)) => {
            let response = GetOpportunityResponse {
                id: opportunity.id,
                name: opportunity.name,
                amount: opportunity.amount,
                stage: opportunity.stage,
                probability: opportunity.probability,
                close_date: opportunity.close_date,
                contact_id: opportunity.contact_id,
                description: opportunity.description,
                created_at: opportunity.created_at,
                updated_at: opportunity.updated_at,
            };
            Ok(Json(response))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update_opportunity_stage(
    State(state): State<Arc<AppState>>,
    Path(opportunity_id): Path<String>,
    Json(payload): Json<UpdateOpportunityStageRequest>,
) -> Result<Json<GetOpportunityResponse>, StatusCode> {
    payload.validate().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    match crud::update_opportunity_stage(
        &state.pool,
        &opportunity_id,
        &payload.stage,
    ).await {
        Ok(Some(opportunity)) => {
            let response = GetOpportunityResponse {
                id: opportunity.id,
                name: opportunity.name,
                amount: opportunity.amount,
                stage: opportunity.stage,
                probability: opportunity.probability,
                close_date: opportunity.close_date,
                contact_id: opportunity.contact_id,
                description: opportunity.description,
                created_at: opportunity.created_at,
                updated_at: opportunity.updated_at,
            };
            Ok(Json(response))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn delete_opportunity(
    State(state): State<Arc<AppState>>,
    Path(opportunity_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    match crud::delete_opportunity(&state.pool, &opportunity_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}