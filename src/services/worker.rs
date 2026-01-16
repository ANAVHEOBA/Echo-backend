use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use crate::AppState;
use super::queue::{Queue, Job};

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
            _ => {
                tracing::warn!("Unknown job type: {}", job.task_type);
                Ok(())
            }
        }
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