use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use crate::AppState;
use super::queue::{Queue, Job};
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::Value;

pub struct Worker {
    queue: Queue,
    state: Arc<AppState>,
}

impl Worker {
    pub fn new(state: Arc<AppState>) -> Self {
        let queue = Queue::new(state.redis.clone());
        Self { queue, state }
    }

    pub async fn run(&self) {
        tracing::info!("Worker started");
        loop {
            match self.queue.dequeue().await {
                Ok(Some(job)) => {
                    tracing::info!("Processing job: {} (type: {})", job.id, job.task_type);
                    if let Err(e) = self.process_job(job).await {
                        tracing::error!("Job failed: {}", e);
                    }
                }
                Ok(None) => {
                    sleep(Duration::from_secs(1)).await;
                }
                Err(e) => {
                    tracing::error!("Queue error: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn process_job(&self, job: Job) -> anyhow::Result<()> {
        match job.task_type.as_str() {
            "email_delivery" => self.handle_email_delivery(job).await,
            "event.process.slack" => self.handle_event_processing(job).await,
            "event.process.gmail" => self.handle_gmail_processing(job).await,
            _ => {
                tracing::warn!("Unknown job type: {}", job.task_type);
                Ok(())
            }
        }
    }
    
    async fn handle_event_processing(&self, job: Job) -> anyhow::Result<()> {
        use crate::modules::events::services::processing::process_slack_event;
        
        let event_id_str = job.payload.get("event_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing event_id in job payload"))?;
        
        let event_id = Uuid::parse_str(event_id_str)
            .map_err(|e| anyhow::anyhow!("Invalid event_id UUID: {}", e))?;
        
        tracing::info!("Processing Slack event: {}", event_id);
        
        process_slack_event(event_id, &self.state.pool).await?;
        
        tracing::info!("Successfully processed Slack event: {}", event_id);
        Ok(())
    }

    async fn handle_gmail_processing(&self, job: Job) -> anyhow::Result<()> {
        use crate::modules::events::crud;
        use crate::modules::events::services::gmail::GmailService;
        use crate::modules::events::services::extractors::extract_business_data;
        use crate::modules::crm_integration::models::Opportunity;

        let event_id_str = job.payload.get("event_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing event_id"))?;
        
        let event_id = Uuid::parse_str(event_id_str)?;
        let event = crud::get_event_by_id(&self.state.pool, &event_id).await?
            .ok_or_else(|| anyhow::anyhow!("Event not found"))?;

        // 1. Decode Google Pub/Sub data to get emailAddress
        let message_data_b64 = event.payload.get("message")
            .and_then(|m| m.get("data"))
            .and_then(|d| d.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing message data in payload"))?;
            
        let decoded_bytes = BASE64.decode(message_data_b64)?;
        let decoded_str = String::from_utf8(decoded_bytes)?;
        let pubsub_data: Value = serde_json::from_str(&decoded_str)?;
        
        let email_address = pubsub_data.get("emailAddress")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing emailAddress in Pub/Sub data"))?;
            
        tracing::info!("Processing Gmail event for: {}", email_address);

        // 2. Find User and OAuth Connection
        // Find user by email - Using runtime query to avoid macro issues
        let user_row = sqlx::query("SELECT id FROM users WHERE email = $1")
            .bind(email_address)
            .fetch_optional(&self.state.pool)
            .await?;
            
        let user_id: Uuid = match user_row {
            Some(row) => {
                use sqlx::Row;
                row.get("id")
            },
            None => {
                tracing::warn!("No user found for email {}, skipping", email_address);
                return Ok(());
            }
        };

        // 3. Get Access Token (Refresh if needed)
        let connection_row = sqlx::query(
            r#"SELECT access_token FROM oauth_connections WHERE user_id = $1 AND provider = 'google'"#
        )
        .bind(user_id)
        .fetch_optional(&self.state.pool)
        .await?;
        
        let access_token: String = match connection_row {
            Some(row) => {
                use sqlx::Row;
                row.get("access_token")
            },
            None => {
                tracing::error!("No Google connection for user {}", user_id);
                return Ok(());
            }
        };

        // 4. Fetch Recent Emails
        let gmail_service = GmailService::new();
        tracing::info!("Fetching recent emails for user {}", user_id);
        
        // Fetch only 1 most recent message for now
        let messages = gmail_service.list_messages(&access_token, None, Some(1)).await?;
        
        if messages.is_empty() {
            tracing::info!("No recent messages found for user {}", user_id);
            return Ok(());
        }
        
        // Process the most recent message
        let msg_summary = &messages[0];
        tracing::info!("Fetching message details for ID: {}", msg_summary.id);
        
        let message = gmail_service.get_message(&access_token, &msg_summary.id).await?;
        
        // 5. Extract Data & Update CRM
        if let Some(body) = gmail_service.extract_body(&message) {
            tracing::info!("Extracted body length: {}", body.len());
            
            let extracted = extract_business_data(&body);
            tracing::debug!("Extracted data: {:?}", extracted);
            
            if extracted.deal_amount.is_some() || extracted.company_name.is_some() {
                // Determine contact ID (placeholder or find by sender email)
                let contact_id = "system".to_string(); 
                
                // Create opportunity name
                let opportunity_name = match (&extracted.company_name, &extracted.deal_amount) {
                    (Some(company), Some(amount)) => format!("{} - ${}", company, amount),
                    (Some(company), None) => format!("{} - New Deal", company),
                    (None, Some(amount)) => format!("Deal - ${}", amount),
                    (None, None) => "New Opportunity from Email".to_string(),
                };
                
                let mut opportunity = Opportunity::new(opportunity_name, contact_id);
                opportunity.amount = extracted.deal_amount.map(|a| a as f64);
                opportunity.stage = extracted.stage.clone().unwrap_or_else(|| "New".to_string());
                opportunity.description = Some(format!("From email: {}\n\n{}", message.snippet, body));
                
                // Insert into DB
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
                .execute(&self.state.pool)
                .await?;
                
                tracing::info!("Created opportunity from email: {} ({})", opportunity.name, opportunity.id);
            }
        }
        
        // Mark event as processed
        crud::mark_event_processed(&self.state.pool, &event_id).await?;
        
        Ok(())
    }

    async fn handle_email_delivery(&self, job: Job) -> anyhow::Result<()> {
        let to = job.payload.get("to")
            .or_else(|| job.payload.get("email"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing recipient email in payload"))?;
            
        let subject = job.payload.get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("Echo Notification");
            
        let body = job.payload.get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("Default message body.");

        tracing::info!("Sending real email to {} via SMTP", to);
        
        let result: Result<(), crate::errors::ApiError> = self.state.email_service.send_email(to, subject, body).await;
        result.map_err(|e| anyhow::anyhow!("Email delivery failed: {}", e))?;

        tracing::info!("Email successfully sent to {}", to);
        Ok(())
    }
}