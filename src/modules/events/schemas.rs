use serde::{Deserialize, Serialize};
use validator::Validate;

// Slack webhook payload schemas
#[derive(Debug, Deserialize, Serialize)]
pub struct SlackWebhookPayload {
    #[serde(rename = "type")]
    pub event_type: String,
    pub token: Option<String>,
    pub challenge: Option<String>, // For URL verification
    pub event: Option<serde_json::Value>,
    pub event_id: Option<String>,
    pub team_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SlackChallengeResponse {
    pub challenge: String,
}

// Gmail push notification payload
#[derive(Debug, Deserialize, Serialize)]
pub struct GmailPushPayload {
    pub message: GmailPushMessage,
    pub subscription: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GmailPushMessage {
    pub data: String, // Base64 encoded
    #[serde(alias = "messageId", alias = "message_id")]
    pub message_id: String,
    #[serde(alias = "publishTime", alias = "publish_time")]
    pub publish_time: String,
}

// Zoom webhook payload
#[derive(Debug, Deserialize, Serialize)]
pub struct ZoomWebhookPayload {
    pub event: String,
    pub payload: serde_json::Value,
    pub event_ts: Option<i64>,
}

// Generic webhook payload
#[derive(Debug, Deserialize, Serialize)]
pub struct GenericWebhookPayload {
    pub event: String,
    pub data: Option<serde_json::Value>,
    pub timestamp: Option<String>,
}

// Event query filters
#[derive(Debug, Deserialize, Default)]
pub struct EventFilter {
    pub event_type: Option<String>,
    pub source: Option<String>,
    pub processed: Option<bool>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

// Response schemas
#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub id: String,
    pub event_type: String,
    pub source: String,
    pub external_id: Option<String>,
    pub payload: serde_json::Value,
    pub processed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct EventStatsResponse {
    pub total_events: i64,
    pub processed_events: i64,
    pub pending_events: i64,
    pub failed_events: i64,
}

// Subscription schemas
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSubscriptionRequest {
    #[validate(length(min = 1))]
    pub platform: String,
    #[validate(url)]
    pub webhook_url: String,
    #[validate(length(min = 8))]
    pub secret: String,
    pub event_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub id: String,
    pub platform: String,
    pub webhook_url: String,
    pub active: bool,
    pub created_at: String,
}
