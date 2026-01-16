use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, AsyncSmtpTransport, Tokio1Executor, AsyncTransport};
use crate::config::AppConfig;
use crate::errors::ApiError;

#[async_trait]
pub trait EmailService: Send + Sync {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), ApiError>;
}

pub struct SmtpEmailService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
}

impl SmtpEmailService {
    pub fn new(config: &AppConfig) -> Self {
        let creds = Credentials::new(
            config.smtp_username.clone(),
            config.smtp_password().to_string(),
        );

        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
            .expect("Failed to create SMTP transport")
            .port(config.smtp_port)
            .credentials(creds)
            .build();

        Self {
            transport,
            from_email: config.email_from.clone(),
        }
    }
}

#[async_trait]
impl EmailService for SmtpEmailService {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), ApiError> {
        let email = Message::builder()
            .from(self.from_email.parse().map_err(|_| ApiError::InternalError("Invalid from email".to_string()))?)
            .to(to.parse().map_err(|_| ApiError::InternalError("Invalid recipient email".to_string()))?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
            .map_err(|e| ApiError::InternalError(format!("Failed to build email: {}", e)))?;

        self.transport
            .send(email)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to send email: {}", e)))?;

        Ok(())
    }
}

// Mock Email Service for testing
pub struct MockEmailService;

#[async_trait]
impl EmailService for MockEmailService {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        _body: &str,
    ) -> Result<(), ApiError> {
        tracing::info!("MOCK EMAIL: To: {}, Subject: {}", to, subject);
        Ok(())
    }
}