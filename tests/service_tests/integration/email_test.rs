use echo_backend::services::email::{EmailService, SmtpEmailService};
use crate::common::create_test_config;

#[tokio::test]
async fn test_smtp_email_send_real() {
    let config = create_test_config().await;
    
    // Skip if using default test password (common in CI)
    if config.smtp_password() == "test_password" {
        tracing::warn!("Skipping real SMTP test: default password detected");
        return;
    }

    let service = SmtpEmailService::new(&config);
    
    // Send to ourselves
    let to = &config.email_from;
    let subject = "Echo System Test";
    let body = "<html><body><h1>Verification Successful</h1><p>The Echo Email Service is correctly configured.</p></body></html>";

    tracing::info!("Attempting to send real test email to {}", to);
    let result = service.send_email(to, subject, body).await;
    
    assert!(result.is_ok(), "Real email send failed: {:?}", result.err());
}
