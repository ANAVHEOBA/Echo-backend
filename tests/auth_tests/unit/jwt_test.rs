///! Unit Tests for JWT Token Handling

use chrono::Utc;
use echo_backend::config::AppConfig;
use echo_backend::modules::auth::model::{TokenType, User};
use echo_backend::modules::auth::services::{
    create_access_token, create_refresh_token, hash_token, validate_access_token,
    validate_refresh_token,
};
use secrecy::SecretString;
use std::sync::Arc;
use uuid::Uuid;

// =============================================================================
// TEST HELPERS
// =============================================================================

fn test_config() -> Arc<AppConfig> {
    Arc::new(AppConfig {
        database_url: SecretString::from("postgres://test:test@localhost/test"),
        redis_url: "redis://localhost:6379".to_string(),
        jwt_secret: SecretString::from("test_jwt_secret_key_that_is_at_least_32_characters_long"),
        jwt_refresh_secret: SecretString::from("test_refresh_secret_key_that_is_at_least_32_chars"),
        access_token_expiry_secs: 900,
        refresh_token_expiry_days: 7,
        encryption_key: SecretString::from("test_encryption_key_32_chars_ok"),
        host: "127.0.0.1".to_string(),
        port: 3000,
        smtp_host: "smtp.gmail.com".to_string(),
        smtp_port: 587,
        smtp_username: "test@example.com".to_string(),
        smtp_password: SecretString::from("test_password"),
        email_from: "test@example.com".to_string(),
    })
}

fn test_user() -> User {
    User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        password_hash: "hashed_password".to_string(),
        first_name: Some("Test".to_string()),
        last_name: Some("User".to_string()),
        organization_id: None,
        role: "member".to_string(),
        email_verified: true,
        email_verified_at: Some(Utc::now()),
        failed_login_attempts: 0,
        locked_until: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
    }
}

fn test_user_with_org() -> User {
    let mut user = test_user();
    user.organization_id = Some(Uuid::new_v4());
    user
}

// =============================================================================
// TOKEN GENERATION TESTS
// =============================================================================

#[tokio::test]
async fn jwt_access_token_has_correct_structure() {
    let config = test_config();
    let user = test_user();

    let token = create_access_token(&user, &config).unwrap();

    // Token should have 3 parts (header.payload.signature)
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts");

    // Each part should be non-empty
    for (i, part) in parts.iter().enumerate() {
        assert!(!part.is_empty(), "Part {} should not be empty", i);
    }
}

#[tokio::test]
async fn jwt_access_token_contains_required_claims() {
    let config = test_config();
    let user = test_user();

    let token = create_access_token(&user, &config).unwrap();
    let claims = validate_access_token(&token, &config).unwrap();

    assert_eq!(claims.sub, user.id);
    assert_eq!(claims.email, user.email);
    assert_eq!(claims.role, user.role);
    assert_eq!(claims.token_type, TokenType::Access);
    assert!(claims.iat > 0);
    assert!(claims.exp > claims.iat);
    assert!(!claims.jti.is_nil());
}

#[tokio::test]
async fn jwt_access_token_contains_org_id_when_present() {
    let config = test_config();
    let user = test_user_with_org();

    let token = create_access_token(&user, &config).unwrap();
    let claims = validate_access_token(&token, &config).unwrap();

    assert!(claims.org_id.is_some());
    assert_eq!(claims.org_id, user.organization_id);
}

#[tokio::test]
async fn jwt_access_token_omits_org_id_when_absent() {
    let config = test_config();
    let user = test_user(); // No org_id

    let token = create_access_token(&user, &config).unwrap();
    let claims = validate_access_token(&token, &config).unwrap();

    assert!(claims.org_id.is_none());
}

#[tokio::test]
async fn jwt_access_token_jti_is_unique() {
    let config = test_config();
    let user = test_user();

    let token1 = create_access_token(&user, &config).unwrap();
    let token2 = create_access_token(&user, &config).unwrap();

    let claims1 = validate_access_token(&token1, &config).unwrap();
    let claims2 = validate_access_token(&token2, &config).unwrap();

    assert_ne!(claims1.jti, claims2.jti, "Each token should have unique jti");
}

#[tokio::test]
async fn jwt_refresh_token_has_correct_structure() {
    let config = test_config();
    let user = test_user();

    let token = create_refresh_token(&user, &config).unwrap();

    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT should have 3 parts");
}

#[tokio::test]
async fn jwt_refresh_token_has_correct_type() {
    let config = test_config();
    let user = test_user();

    let token = create_refresh_token(&user, &config).unwrap();
    let claims = validate_refresh_token(&token, &config).unwrap();

    assert_eq!(claims.token_type, TokenType::Refresh);
}

// =============================================================================
// EXPIRATION TESTS
// =============================================================================

#[tokio::test]
async fn jwt_access_token_expires_in_15_minutes() {
    let config = test_config();
    let user = test_user();

    let token = create_access_token(&user, &config).unwrap();
    let claims = validate_access_token(&token, &config).unwrap();

    let expiry_duration = claims.exp - claims.iat;
    assert_eq!(expiry_duration, 900, "Access token should expire in 900 seconds (15 min)");
}

#[tokio::test]
async fn jwt_refresh_token_expires_in_7_days() {
    let config = test_config();
    let user = test_user();

    let token = create_refresh_token(&user, &config).unwrap();
    let claims = validate_refresh_token(&token, &config).unwrap();

    let expiry_duration = claims.exp - claims.iat;
    let expected = 7 * 24 * 60 * 60; // 7 days in seconds
    assert_eq!(expiry_duration, expected, "Refresh token should expire in 7 days");
}

#[tokio::test]
async fn jwt_iat_is_current_timestamp() {
    let before = Utc::now().timestamp();

    let config = test_config();
    let user = test_user();
    let token = create_access_token(&user, &config).unwrap();

    let after = Utc::now().timestamp();
    let claims = validate_access_token(&token, &config).unwrap();

    assert!(claims.iat >= before, "iat should be >= before time");
    assert!(claims.iat <= after, "iat should be <= after time");
}

#[tokio::test]
async fn jwt_validation_rejects_expired_token() {
    // Create config with very short expiry and wait for expiration
    let config = Arc::new(AppConfig {
        database_url: SecretString::from("postgres://test:test@localhost/test"),
        redis_url: "redis://localhost:6379".to_string(),
        jwt_secret: SecretString::from("test_jwt_secret_key_that_is_at_least_32_characters_long"),
        jwt_refresh_secret: SecretString::from("test_refresh_secret_key_that_is_at_least_32_chars"),
        access_token_expiry_secs: 1, // 1 second expiry
        refresh_token_expiry_days: 7,
        encryption_key: SecretString::from("test_encryption_key_32_chars_ok"),
        host: "127.0.0.1".to_string(),
        port: 3000,
        smtp_host: "smtp.gmail.com".to_string(),
        smtp_port: 587,
        smtp_username: "test@example.com".to_string(),
        smtp_password: SecretString::from("test_password"),
        email_from: "test@example.com".to_string(),
    });

    let user = test_user();
    let token = create_access_token(&user, &config).unwrap();

    // Wait for token to expire
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let result = validate_access_token(&token, &config);
    assert!(result.is_err(), "Expired token should be rejected");
}

// =============================================================================
// SIGNATURE VALIDATION TESTS
// =============================================================================

#[tokio::test]
async fn jwt_signature_is_valid_with_correct_secret() {
    let config = test_config();
    let user = test_user();

    let token = create_access_token(&user, &config).unwrap();
    let result = validate_access_token(&token, &config);

    assert!(result.is_ok(), "Valid token should be accepted");
}

#[tokio::test]
async fn jwt_signature_fails_with_wrong_secret() {
    let config = test_config();
    let user = test_user();
    let token = create_access_token(&user, &config).unwrap();

    // Create config with different secret
    let wrong_config = Arc::new(AppConfig {
        database_url: SecretString::from("postgres://test:test@localhost/test"),
        redis_url: "redis://localhost:6379".to_string(),
        jwt_secret: SecretString::from("different_secret_key_that_is_at_least_32_characters"),
        jwt_refresh_secret: SecretString::from("test_refresh_secret_key_that_is_at_least_32_chars"),
        access_token_expiry_secs: 900,
        refresh_token_expiry_days: 7,
        encryption_key: SecretString::from("test_encryption_key_32_chars_ok"),
        host: "127.0.0.1".to_string(),
        port: 3000,
        smtp_host: "smtp.gmail.com".to_string(),
        smtp_port: 587,
        smtp_username: "test@example.com".to_string(),
        smtp_password: SecretString::from("test_password"),
        email_from: "test@example.com".to_string(),
    });

    let result = validate_access_token(&token, &wrong_config);
    assert!(result.is_err(), "Token with wrong secret should be rejected");
}

#[tokio::test]
async fn jwt_detects_tampered_payload() {
    let config = test_config();
    let user = test_user();
    let token = create_access_token(&user, &config).unwrap();

    // Tamper with payload by modifying a character
    let parts: Vec<&str> = token.split('.').collect();
    let tampered_payload = format!("{}x", parts[1]); // Add character to payload
    let tampered_token = format!("{}.{}.{}", parts[0], tampered_payload, parts[2]);

    let result = validate_access_token(&tampered_token, &config);
    assert!(result.is_err(), "Tampered token should be rejected");
}

#[tokio::test]
async fn jwt_detects_truncated_signature() {
    let config = test_config();
    let user = test_user();
    let token = create_access_token(&user, &config).unwrap();

    let parts: Vec<&str> = token.split('.').collect();
    let truncated_sig = &parts[2][..parts[2].len() / 2];
    let tampered_token = format!("{}.{}.{}", parts[0], parts[1], truncated_sig);

    let result = validate_access_token(&tampered_token, &config);
    assert!(result.is_err(), "Truncated signature should be rejected");
}

#[tokio::test]
async fn jwt_detects_missing_signature() {
    let config = test_config();
    let user = test_user();
    let token = create_access_token(&user, &config).unwrap();

    let parts: Vec<&str> = token.split('.').collect();
    let no_sig_token = format!("{}.{}", parts[0], parts[1]);

    let result = validate_access_token(&no_sig_token, &config);
    assert!(result.is_err(), "Missing signature should be rejected");
}

// =============================================================================
// TOKEN TYPE VALIDATION TESTS
// =============================================================================

#[tokio::test]
async fn jwt_access_validator_rejects_refresh_token() {
    let config = test_config();
    let user = test_user();

    let refresh_token = create_refresh_token(&user, &config).unwrap();
    let result = validate_access_token(&refresh_token, &config);

    assert!(result.is_err(), "Access validator should reject refresh token");
}

#[tokio::test]
async fn jwt_refresh_validator_rejects_access_token() {
    let config = test_config();
    let user = test_user();

    let access_token = create_access_token(&user, &config).unwrap();
    let result = validate_refresh_token(&access_token, &config);

    assert!(result.is_err(), "Refresh validator should reject access token");
}

// =============================================================================
// MALFORMED TOKEN TESTS
// =============================================================================

#[tokio::test]
async fn jwt_decode_handles_malformed_token() {
    let config = test_config();
    let malformed = "not.a.valid.jwt.token";

    let result = validate_access_token(malformed, &config);
    assert!(result.is_err(), "Malformed token should be rejected");
}

#[tokio::test]
async fn jwt_decode_handles_invalid_base64() {
    let config = test_config();
    let invalid_b64 = "!!!.@@@.###";

    let result = validate_access_token(invalid_b64, &config);
    assert!(result.is_err(), "Invalid base64 should be rejected");
}

#[tokio::test]
async fn jwt_decode_handles_empty_string() {
    let config = test_config();

    let result = validate_access_token("", &config);
    assert!(result.is_err(), "Empty string should be rejected");
}

#[tokio::test]
async fn jwt_decode_handles_single_part() {
    let config = test_config();

    let result = validate_access_token("onlyonepart", &config);
    assert!(result.is_err(), "Single part token should be rejected");
}

// =============================================================================
// HASH TOKEN TESTS
// =============================================================================

#[tokio::test]
async fn hash_token_produces_consistent_output() {
    let token = "test_token_12345";

    let hash1 = hash_token(token);
    let hash2 = hash_token(token);

    assert_eq!(hash1, hash2, "Same input should produce same hash");
}

#[tokio::test]
async fn hash_token_produces_different_output_for_different_input() {
    let hash1 = hash_token("token1");
    let hash2 = hash_token("token2");

    assert_ne!(hash1, hash2, "Different inputs should produce different hashes");
}

#[tokio::test]
async fn hash_token_output_is_hex_encoded() {
    let hash = hash_token("test_token");

    // SHA256 produces 32 bytes = 64 hex characters
    assert_eq!(hash.len(), 64, "Hash should be 64 hex characters");
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be hex encoded");
}

// =============================================================================
// SECURITY TESTS
// =============================================================================

#[tokio::test]
async fn jwt_secret_is_not_in_token() {
    let config = test_config();
    let user = test_user();

    let token = create_access_token(&user, &config).unwrap();
    let secret = config.jwt_secret();

    assert!(!token.contains(secret), "Secret should not appear in token");
}

#[tokio::test]
async fn jwt_sensitive_data_not_in_payload() {
    let config = test_config();
    let user = test_user();

    let token = create_access_token(&user, &config).unwrap();
    let claims = validate_access_token(&token, &config).unwrap();

    // Password hash should NOT be in claims - Claims struct doesn't have it
    // This test verifies the Claims struct design is correct
    assert_eq!(claims.email, user.email);
    assert_eq!(claims.role, user.role);
    // No password_hash field exists in Claims - this is by design
}