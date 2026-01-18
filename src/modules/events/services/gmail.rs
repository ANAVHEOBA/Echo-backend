use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct GmailWatchResponse {
    #[serde(alias = "historyId")]
    pub history_id: String,
    pub expiration: String,
}

#[derive(Debug, Deserialize)]
pub struct GmailMessageListResponse {
    pub messages: Option<Vec<GmailMessageSummary>>,
    #[serde(alias = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(alias = "resultSizeEstimate")]
    pub result_size_estimate: u32,
}

#[derive(Debug, Deserialize)]
pub struct GmailMessageSummary {
    pub id: String,
    #[serde(alias = "threadId")]
    pub thread_id: String,
}

#[derive(Debug, Deserialize)]
pub struct GmailMessage {
    pub id: String,
    pub snippet: String,
    pub payload: MessagePayload,
}

#[derive(Debug, Deserialize)]
pub struct MessagePayload {
    pub headers: Vec<MessageHeader>,
    pub body: MessageBody,
    pub parts: Option<Vec<MessagePart>>,
}

#[derive(Debug, Deserialize)]
pub struct MessageHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct MessageBody {
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessagePart {
    #[serde(alias = "mimeType")]
    pub mime_type: String,
    pub body: MessageBody,
    pub parts: Option<Vec<MessagePart>>,
}

pub struct GmailService {
    client: Client,
}

impl GmailService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Set up push notifications for the user's mailbox
    pub async fn watch_mailbox(&self, access_token: &str, topic_name: &str) -> Result<GmailWatchResponse> {
        let url = "https://gmail.googleapis.com/gmail/v1/users/me/watch";
        
        let body = serde_json::json!({
            "topicName": topic_name,
            "labelIds": ["INBOX"]
        });

        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Gmail API error: {}", error_text));
        }

        let watch_response: GmailWatchResponse = response.json().await?;
        Ok(watch_response)
    }

    /// List messages matching a query
    pub async fn list_messages(
        &self, 
        access_token: &str, 
        query: Option<&str>, 
        max_results: Option<u32>
    ) -> Result<Vec<GmailMessageSummary>> {
        let base_url = "https://gmail.googleapis.com/gmail/v1/users/me/messages";
        let mut url = Url::parse(base_url).map_err(|e| anyhow!("Invalid URL: {}", e))?;
        
        {
            let mut pairs = url.query_pairs_mut();
            if let Some(q) = query {
                pairs.append_pair("q", q);
            }
            if let Some(max) = max_results {
                pairs.append_pair("maxResults", &max.to_string());
            }
        }

        let response = self.client.get(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Gmail API error: {}", error_text));
        }

        let list_response: GmailMessageListResponse = response.json().await?;
        Ok(list_response.messages.unwrap_or_default())
    }

    /// Fetch a full email message by ID
    pub async fn get_message(&self, access_token: &str, message_id: &str) -> Result<GmailMessage> {
        let url = format!(
            "https://gmail.googleapis.com/gmail/v1/users/me/messages/{}",
            message_id
        );

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Gmail API error: {}", error_text));
        }

        let message: GmailMessage = response.json().await?;
        Ok(message)
    }

    /// Extract the plain text body from a Gmail message
    pub fn extract_body(&self, message: &GmailMessage) -> Option<String> {
        // Helper to recursively search for text/plain parts
        fn find_plain_text(parts: &[MessagePart]) -> Option<String> {
            for part in parts {
                if part.mime_type == "text/plain" {
                    if let Some(data) = &part.body.data {
                        return Some(data.clone());
                    }
                }
                if let Some(sub_parts) = &part.parts {
                    if let Some(text) = find_plain_text(sub_parts) {
                        return Some(text);
                    }
                }
            }
            None
        }

        // Check main body first
        if let Some(data) = &message.payload.body.data {
            return self.decode_body(data);
        }

        // Check parts
        if let Some(parts) = &message.payload.parts {
            if let Some(encoded) = find_plain_text(parts) {
                return self.decode_body(&encoded);
            }
        }

        Some(message.snippet.clone()) // Fallback to snippet
    }

    fn decode_body(&self, data: &str) -> Option<String> {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE};
        match URL_SAFE.decode(data) {
            Ok(bytes) => String::from_utf8(bytes).ok(),
            Err(_) => None,
        }
    }
}
