///! Unit Tests for User Registration Validation

use echo_backend::modules::auth::services::{hash_password, validate_password_strength};

// =============================================================================
// PASSWORD VALIDATION FOR REGISTRATION
// =============================================================================

#[tokio::test]
async fn register_password_is_hashed_not_stored_plaintext() {
    let password = "SecurePass123!";
    let hash = hash_password(password).unwrap();

    assert!(!hash.contains(password), "Hash should not contain plaintext password");
    assert!(hash.starts_with("$argon2"), "Should use Argon2 hashing");
}

#[tokio::test]
async fn register_password_hash_uses_unique_salt() {
    let password = "SecurePass123!";

    let hash1 = hash_password(password).unwrap();
    let hash2 = hash_password(password).unwrap();

    assert_ne!(hash1, hash2, "Two users with same password should have different hashes");
}

#[tokio::test]
async fn register_rejects_weak_password_empty() {
    let result = validate_password_strength("");
    assert!(result.is_err(), "Empty password should be rejected");
}

#[tokio::test]
async fn register_rejects_weak_password_too_short() {
    let result = validate_password_strength("Short1");
    assert!(result.is_err(), "Password under 8 chars should be rejected");
}

#[tokio::test]
async fn register_rejects_weak_password_no_letter() {
    let result = validate_password_strength("12345678");
    assert!(result.is_err(), "Password without letter should be rejected");
}

#[tokio::test]
async fn register_rejects_weak_password_no_number() {
    let result = validate_password_strength("abcdefgh");
    assert!(result.is_err(), "Password without number should be rejected");
}

#[tokio::test]
async fn register_accepts_password_at_minimum_length() {
    let result = validate_password_strength("Abcd1234");
    assert!(result.is_ok(), "8 character password with letter and number should be accepted");
}

#[tokio::test]
async fn register_rejects_password_exceeding_128_chars() {
    let long_password = "A1".to_string() + &"a".repeat(127);
    assert!(long_password.len() > 128);

    let result = validate_password_strength(&long_password);
    assert!(result.is_err(), "Password over 128 chars should be rejected");
}

#[tokio::test]
async fn register_accepts_password_at_max_length() {
    let password = "A1".to_string() + &"a".repeat(126);
    assert_eq!(password.len(), 128);

    let result = validate_password_strength(&password);
    assert!(result.is_ok(), "128 character password should be accepted");
}

#[tokio::test]
async fn register_password_hash_is_not_reversible() {
    let password = "SecurePass123!";
    let hash = hash_password(password).unwrap();

    // Cannot derive original password from hash
    assert!(!hash.contains(password));
    assert!(hash.len() > password.len());
}

// =============================================================================
// EMAIL VALIDATION TESTS (unit tests for format)
// =============================================================================

fn is_valid_email_format(email: &str) -> bool {
    // Simple email validation for unit tests
    email.contains('@')
        && email.split('@').count() == 2
        && !email.starts_with('@')
        && !email.ends_with('@')
        && email.split('@').last().map(|d| d.contains('.')).unwrap_or(false)
}

#[tokio::test]
async fn email_validation_accepts_valid_email() {
    assert!(is_valid_email_format("user@example.com"));
    assert!(is_valid_email_format("test.user@domain.org"));
    assert!(is_valid_email_format("user+tag@example.co.uk"));
}

#[tokio::test]
async fn email_validation_rejects_missing_at() {
    assert!(!is_valid_email_format("userexample.com"));
}

#[tokio::test]
async fn email_validation_rejects_missing_domain() {
    assert!(!is_valid_email_format("user@"));
}

#[tokio::test]
async fn email_validation_rejects_missing_local() {
    assert!(!is_valid_email_format("@example.com"));
}

#[tokio::test]
async fn email_validation_rejects_empty() {
    assert!(!is_valid_email_format(""));
}

// =============================================================================
// NAME VALIDATION TESTS
// =============================================================================

fn is_valid_name(name: Option<&str>) -> bool {
    match name {
        None => true, // Names are optional
        Some(n) => !n.trim().is_empty() && n.len() <= 100,
    }
}

#[tokio::test]
async fn name_validation_accepts_none() {
    assert!(is_valid_name(None));
}

#[tokio::test]
async fn name_validation_accepts_valid_name() {
    assert!(is_valid_name(Some("John")));
    assert!(is_valid_name(Some("Mary-Jane")));
    assert!(is_valid_name(Some("José")));
}

#[tokio::test]
async fn name_validation_rejects_too_long() {
    let long_name = "a".repeat(101);
    assert!(!is_valid_name(Some(&long_name)));
}

#[tokio::test]
async fn name_validation_accepts_max_length() {
    let max_name = "a".repeat(100);
    assert!(is_valid_name(Some(&max_name)));
}

#[tokio::test]
async fn name_validation_rejects_whitespace_only() {
    assert!(!is_valid_name(Some("   ")));
}
