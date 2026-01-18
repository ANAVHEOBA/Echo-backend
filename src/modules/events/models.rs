use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub source: String,
    pub external_id: Option<String>,
    pub payload: serde_json::Value,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Event {
    pub fn new(
        event_type: String,
        source: String,
        external_id: Option<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            source,
            external_id,
            payload,
            processed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct EventLog {
    pub id: Uuid,
    pub event_id: Uuid,
    pub worker_type: String,
    pub status: String,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub platform: String,
    pub webhook_url: String,
    pub secret: String,
    pub event_types: Option<serde_json::Value>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
