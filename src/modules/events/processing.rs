use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::config::DbPool;
use super::models::Event;
use super::crud;
use super::extractors::{extract_business_data, ExtractedData};
use crate::modules::crm_integration::models::Opportunity;

/// Process a Slack event: extract data and update CRM
pub async fn process_slack_event(
    event_id: Uuid,
    pool: &DbPool,
) -> Result<()> {
    // Get event from database
    let event = crud::get_event_by_id(pool, &event_id).await?
        .ok_or_else(|| anyhow!("Event not found: {}", event_id))?;
    
    // Skip if already processed
    if event.processed_at.is_some() {
        tracing::info!("Event {} already processed, skipping", event_id);
        return Ok(());
    }
    
    tracing::info!("Processing Slack event: {}", event_id);
    
    // Extract message text from payload
    let message_text = extract_message_from_payload(&event)?;
    
    // Extract business data using patterns
    let extracted = extract_business_data(&message_text);
    
    tracing::debug!("Extracted data: {:?}", extracted);
    
    // Update CRM based on extracted data
    if extracted.deal_amount.is_some() || extracted.company_name.is_some() {
        update_crm_from_extracted(pool, &extracted).await?;
    }
    
    // Mark event as processed
    crud::mark_event_processed(pool, &event_id).await?;
    
    tracing::info!("Successfully processed event: {}", event_id);
    Ok(())
}

/// Extract message text from Slack event payload
fn extract_message_from_payload(event: &Event) -> Result<String> {
    let payload = &event.payload;
    
    // Extract text from nested event object
    let text = payload
        .get("event")
        .and_then(|e| e.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("No text found in event payload"))?;
    
    Ok(text.to_string())
}

/// Update CRM with extracted data
async fn update_crm_from_extracted(
    pool: &DbPool,
    extracted: &ExtractedData,
) -> Result<()> {
    // Only create opportunity if we have deal amount or company
    if extracted.deal_amount.is_none() && extracted.company_name.is_none() {
        return Ok(());
    }
    
    // Generate opportunity name
    let opportunity_name = match (&extracted.company_name, &extracted.deal_amount) {
        (Some(company), Some(amount)) => format!("{} - ${}", company, amount),
        (Some(company), None) => format!("{} - New Deal", company),
        (None, Some(amount)) => format!("Deal - ${}", amount),
        (None, None) => "New Opportunity".to_string(),
    };
    
    // Create opportunity
    let mut opportunity = Opportunity::new(
        opportunity_name,
        "system".to_string(), // contact_id placeholder
    );
    
    // Set additional fields
    opportunity.amount = extracted.deal_amount.map(|a| a as f64);
    opportunity.stage = extracted.stage.clone().unwrap_or_else(|| "New".to_string());
    opportunity.description = extracted.description.clone();
    
    // Insert into database
    sqlx::query(
        r#"
        INSERT INTO opportunities (id, name, amount, stage, probability, close_date, contact_id, description, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#
    )
   .bind(&opportunity.id)
    .bind(&opportunity.name)
    .bind(&opportunity.amount)
    .bind(&opportunity.stage)
    .bind(&opportunity.probability)
    .bind(&opportunity.close_date)
    .bind(&opportunity.contact_id)
    .bind(&opportunity.description)
    .bind(&opportunity.created_at)
    .bind(&opportunity.updated_at)
    .execute(pool)
    .await?;
    
    tracing::info!("Created opportunity: {} ({})", opportunity.name, opportunity.id);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_message_from_payload() {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "message".to_string(),
            source: "slack".to_string(),
            external_id: Some("test".to_string()),
            payload: json!({
                "event": {
                    "text": "Hello world",
                    "type": "message"
                }
            }),
            processed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        let text = extract_message_from_payload(&event).unwrap();
        assert_eq!(text, "Hello world");
    }
}
