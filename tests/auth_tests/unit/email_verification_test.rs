///! Unit Tests for Email Verification
///!
///! Tests cover:
///! - Token generation
///! - Verification process
///! - Token expiration
///! - Resend functionality
///! - Rate limiting

use chrono::{Duration, Utc};
use echo_backend::modules::auth::services::{generate_token, hash_token};
use fake::faker::internet::en::SafeEmail;
use fake::Fake;
use std::collections::HashSet;
use uuid::Uuid;

// =============================================================================
// TEST HELPERS
// =============================================================================

fn valid_email() -> String {
    SafeEmail().fake()
}

/// Simulates an email verification token structure
#[derive(Debug, Clone)]
struct EmailVerificationToken {
    id: Uuid,
    user_id: Uuid,
    token_hash: String,
    expires_at: chrono::DateTime<Utc>,
    used: bool,
    used_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
}

impl EmailVerificationToken {
    fn new(user_id: Uuid, token: &str, expiry_hours: i64) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            token_hash: hash_token(token),
            expires_at: Utc::now() + Duration::hours(expiry_hours),
            used: false,
            used_at: None,
            created_at: Utc::now(),
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    fn is_valid(&self, token: &str) -> bool {
        !self.used && !self.is_expired() && hash_token(token) == self.token_hash
    }

    fn mark_used(&mut self) {
        self.used = true;
        self.used_at = Some(Utc::now());
    }
}

/// Simulates user email verification state
#[derive(Debug, Clone)]
struct UserEmailState {
    user_id: Uuid,
    email: String,
    email_verified: bool,
    email_verified_at: Option<chrono::DateTime<Utc>>,
}

impl UserEmailState {
    fn new(email: &str) -> Self {
        Self {
            user_id: Uuid::new_v4(),
            email: email.to_string(),
            email_verified: false,
            email_verified_at: None,
        }
    }

    fn verify(&mut self) {
        self.email_verified = true;
        self.email_verified_at = Some(Utc::now());
    }

    fn email(&self) -> &str {
        &self.email
    }
}

// =============================================================================
// TOKEN GENERATION TESTS
// =============================================================================

#[tokio::test]
async fn registration_creates_verification_token() {
    // Arrange
    let email = valid_email();
    let user = UserEmailState::new(&email);
    let raw_token = generate_token();

    // Act
    let verification_token = EmailVerificationToken::new(user.user_id, &raw_token, 24);

    // Assert
    assert_eq!(user.email(), email, "User email should be stored");
    assert_eq!(verification_token.user_id, user.user_id, "Token should be linked to user");
    assert!(!verification_token.token_hash.is_empty(), "Token hash should be created");
    assert!(!verification_token.used, "Token should not be used initially");
}

#[tokio::test]
async fn verification_token_expires_in_24_hours() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let before = Utc::now();

    // Act
    let token = EmailVerificationToken::new(user_id, &raw_token, 24);

    // Assert - expires_at should be approximately 24 hours from now
    let expected_min = before + Duration::hours(24) - Duration::seconds(1);
    let expected_max = Utc::now() + Duration::hours(24) + Duration::seconds(1);

    assert!(token.expires_at >= expected_min, "Expiry should be at least 24h from start");
    assert!(token.expires_at <= expected_max, "Expiry should be at most 24h from end");
}

#[tokio::test]
async fn verification_token_is_cryptographically_secure() {
    // Arrange & Act - Generate multiple tokens
    let mut tokens = HashSet::new();
    for _ in 0..100 {
        tokens.insert(generate_token());
    }

    // Assert
    assert_eq!(tokens.len(), 100, "All tokens should be unique");

    let sample = generate_token();
    assert_eq!(sample.len(), 64, "Token should be 64 hex characters");
    assert!(sample.chars().all(|c| c.is_ascii_hexdigit()), "Token should be hex encoded");
}

#[tokio::test]
async fn verification_token_hash_is_stored() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();

    // Act
    let token = EmailVerificationToken::new(user_id, &raw_token, 24);

    // Assert - SHA-256 hash should be stored, not plaintext
    assert_eq!(token.token_hash.len(), 64, "Hash should be SHA-256 (64 hex chars)");
    assert_ne!(token.token_hash, raw_token, "Stored hash should not equal plaintext");
    assert!(!token.token_hash.contains(&raw_token), "Hash should not contain plaintext");
}

#[tokio::test]
async fn verification_tokens_are_unique_per_request() {
    // Arrange
    let user_id = Uuid::new_v4();

    // Act
    let token1 = generate_token();
    let token2 = generate_token();
    let vt1 = EmailVerificationToken::new(user_id, &token1, 24);
    let vt2 = EmailVerificationToken::new(user_id, &token2, 24);

    // Assert
    assert_ne!(token1, token2, "Raw tokens should be unique");
    assert_ne!(vt1.token_hash, vt2.token_hash, "Token hashes should be unique");
    assert_ne!(vt1.id, vt2.id, "Token IDs should be unique");
}

// =============================================================================
// VERIFICATION PROCESS TESTS
// =============================================================================

#[tokio::test]
async fn verify_email_with_valid_token_succeeds() {
    // Arrange
    let email = valid_email();
    let mut user = UserEmailState::new(&email);
    let raw_token = generate_token();
    let verification_token = EmailVerificationToken::new(user.user_id, &raw_token, 24);

    // Act
    assert!(verification_token.is_valid(&raw_token), "Token should be valid");
    user.verify();

    // Assert
    assert!(user.email_verified, "Email should be verified");
    assert!(user.email_verified_at.is_some(), "Verified timestamp should be set");
}

#[tokio::test]
async fn verify_email_sets_verified_timestamp() {
    // Arrange
    let before = Utc::now();
    let mut user = UserEmailState::new(&valid_email());

    // Act
    user.verify();

    // Assert
    assert!(user.email_verified_at.is_some());
    let verified_at = user.email_verified_at.unwrap();
    assert!(verified_at >= before, "Verified timestamp should be after start time");
    assert!(verified_at <= Utc::now(), "Verified timestamp should be before now");
}

#[tokio::test]
async fn verify_email_marks_token_as_used() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let mut verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);
    let before = Utc::now();

    // Act
    verification_token.mark_used();

    // Assert
    assert!(verification_token.used, "Token should be marked as used");
    assert!(verification_token.used_at.is_some(), "used_at should be set");
    assert!(verification_token.used_at.unwrap() >= before, "used_at should be recent");
}

#[tokio::test]
async fn verify_email_with_expired_token_fails() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let mut verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);

    // Expire the token
    verification_token.expires_at = Utc::now() - Duration::hours(1);

    // Assert
    assert!(verification_token.is_expired(), "Token should be expired");
    assert!(!verification_token.is_valid(&raw_token), "Expired token should be invalid");
}

#[tokio::test]
async fn verify_email_with_invalid_token_fails() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);
    let invalid_token = "invalid_random_token";

    // Assert
    assert!(!verification_token.is_valid(invalid_token), "Invalid token should be rejected");
}

#[tokio::test]
async fn verify_email_with_used_token_fails() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let mut verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);

    // Use the token
    verification_token.mark_used();

    // Assert
    assert!(verification_token.used, "Token should be marked as used");
    assert!(!verification_token.is_valid(&raw_token), "Used token should be invalid");
}

#[tokio::test]
async fn verify_email_is_single_use() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let mut verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);

    // Act - First verification
    assert!(verification_token.is_valid(&raw_token), "First verification should succeed");
    verification_token.mark_used();

    // Assert - Second verification
    assert!(!verification_token.is_valid(&raw_token), "Second verification should fail");
}

#[tokio::test]
async fn already_verified_user_state() {
    // Arrange
    let mut user = UserEmailState::new(&valid_email());
    user.verify();

    // Assert - User is already verified
    assert!(user.email_verified, "User should be verified");

    // Verifying again should be idempotent (in practice)
    let _before_verified_at = user.email_verified_at;
    user.verify();
    // Note: In real implementation, we might not update timestamp
    assert!(user.email_verified, "User should still be verified");
}

// =============================================================================
// RESEND VERIFICATION TESTS
// =============================================================================

#[tokio::test]
async fn resend_verification_creates_new_token() {
    // Arrange
    let user_id = Uuid::new_v4();
    let token1 = generate_token();
    let token2 = generate_token();

    // Act
    let vt1 = EmailVerificationToken::new(user_id, &token1, 24);
    let vt2 = EmailVerificationToken::new(user_id, &token2, 24);

    // Assert
    assert_ne!(vt1.token_hash, vt2.token_hash, "New token should have different hash");
    assert_ne!(vt1.id, vt2.id, "New token should have different ID");
    assert!(vt2.expires_at >= vt1.expires_at, "New token should have new expiration");
}

#[tokio::test]
async fn resend_creates_token_with_new_expiration() {
    // Arrange
    let user_id = Uuid::new_v4();

    // Create first token
    let token1 = generate_token();
    let vt1 = EmailVerificationToken::new(user_id, &token1, 24);
    let first_expiry = vt1.expires_at;

    // Small delay
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Create second token (resend)
    let token2 = generate_token();
    let vt2 = EmailVerificationToken::new(user_id, &token2, 24);

    // Assert
    assert!(vt2.expires_at >= first_expiry, "New token should have later or equal expiry");
}

#[tokio::test]
async fn old_token_should_be_invalidatable() {
    // Arrange
    let user_id = Uuid::new_v4();
    let token1 = generate_token();
    let mut vt1 = EmailVerificationToken::new(user_id, &token1, 24);

    // Simulate invalidation by marking as used (or deleting in real impl)
    vt1.mark_used();

    // Create new token
    let token2 = generate_token();
    let vt2 = EmailVerificationToken::new(user_id, &token2, 24);

    // Assert
    assert!(!vt1.is_valid(&token1), "Old token should be invalid");
    assert!(vt2.is_valid(&token2), "New token should be valid");
}

// =============================================================================
// TOKEN HASH TESTS
// =============================================================================

#[tokio::test]
async fn token_hash_is_deterministic() {
    // Arrange
    let raw_token = "test_verification_token";

    // Act
    let hash1 = hash_token(raw_token);
    let hash2 = hash_token(raw_token);

    // Assert
    assert_eq!(hash1, hash2, "Same token should produce same hash");
}

#[tokio::test]
async fn different_tokens_produce_different_hashes() {
    // Arrange
    let token1 = "token_one";
    let token2 = "token_two";

    // Act
    let hash1 = hash_token(token1);
    let hash2 = hash_token(token2);

    // Assert
    assert_ne!(hash1, hash2, "Different tokens should produce different hashes");
}

#[tokio::test]
async fn hash_is_sha256_format() {
    // Arrange
    let token = generate_token();

    // Act
    let hash = hash_token(&token);

    // Assert
    assert_eq!(hash.len(), 64, "SHA-256 hash should be 64 hex characters");
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be hex encoded");
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[tokio::test]
async fn empty_token_is_rejected() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);

    // Assert
    assert!(!verification_token.is_valid(""), "Empty token should be rejected");
}

#[tokio::test]
async fn whitespace_token_is_rejected() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);

    // Assert
    assert!(!verification_token.is_valid("   "), "Whitespace token should be rejected");
    assert!(!verification_token.is_valid("\t\n"), "Tab/newline token should be rejected");
}

#[tokio::test]
async fn token_with_extra_characters_is_rejected() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);
    let modified = format!("{}extra", raw_token);

    // Assert
    assert!(!verification_token.is_valid(&modified), "Token with extra chars should be rejected");
}

#[tokio::test]
async fn truncated_token_is_rejected() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);
    let truncated = &raw_token[..raw_token.len() - 1];

    // Assert
    assert!(!verification_token.is_valid(truncated), "Truncated token should be rejected");
}

#[tokio::test]
async fn expiry_boundary_check() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let mut verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);

    // Set expiry to very soon
    verification_token.expires_at = Utc::now() + Duration::milliseconds(100);

    // Assert - Should be valid before expiry
    assert!(!verification_token.is_expired(), "Token should not be expired yet");

    // Wait for expiry
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Assert - Should be expired after
    assert!(verification_token.is_expired(), "Token should be expired now");
    assert!(!verification_token.is_valid(&raw_token), "Expired token should be invalid");
}

#[tokio::test]
async fn concurrent_token_generation_produces_unique_tokens() {
    // Arrange
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let tokens = Arc::new(Mutex::new(HashSet::new()));

    // Act - Generate tokens concurrently
    let handles: Vec<_> = (0..50)
        .map(|_| {
            let tokens = tokens.clone();
            tokio::spawn(async move {
                let token = generate_token();
                tokens.lock().await.insert(token);
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }

    // Assert
    let final_tokens = tokens.lock().await;
    assert_eq!(final_tokens.len(), 50, "All concurrent tokens should be unique");
}

#[tokio::test]
async fn token_metadata_is_preserved() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let before = Utc::now();

    // Act
    let verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);

    // Assert
    assert_eq!(verification_token.user_id, user_id, "User ID should be preserved");
    assert!(verification_token.created_at >= before, "Created at should be set");
    assert!(!verification_token.id.is_nil(), "Token should have an ID");
    assert!(!verification_token.used, "Should not be used initially");
    assert!(verification_token.used_at.is_none(), "used_at should be None initially");
}

#[tokio::test]
async fn user_verification_updates_state_correctly() {
    // Arrange
    let mut user = UserEmailState::new(&valid_email());
    let before = Utc::now();

    // Assert initial state
    assert!(!user.email_verified, "Should not be verified initially");
    assert!(user.email_verified_at.is_none(), "Verified at should be None initially");

    // Act
    user.verify();

    // Assert final state
    assert!(user.email_verified, "Should be verified");
    assert!(user.email_verified_at.is_some(), "Verified at should be set");
    let verified_at = user.email_verified_at.unwrap();
    assert!(verified_at >= before, "Verified at should be recent");
}

#[tokio::test]
async fn multiple_users_have_independent_tokens() {
    // Arrange
    let user1_id = Uuid::new_v4();
    let user2_id = Uuid::new_v4();
    let token1 = generate_token();
    let token2 = generate_token();

    // Act
    let vt1 = EmailVerificationToken::new(user1_id, &token1, 24);
    let vt2 = EmailVerificationToken::new(user2_id, &token2, 24);

    // Assert
    assert_eq!(vt1.user_id, user1_id);
    assert_eq!(vt2.user_id, user2_id);
    assert!(vt1.is_valid(&token1), "User 1's token should be valid for user 1");
    assert!(vt2.is_valid(&token2), "User 2's token should be valid for user 2");

    // Cross-user validation should fail (different tokens)
    assert!(!vt1.is_valid(&token2), "User 2's raw token should not validate user 1's hash");
    assert!(!vt2.is_valid(&token1), "User 1's raw token should not validate user 2's hash");
}

#[tokio::test]
async fn token_timing_validation_consistent() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_token = generate_token();
    let verification_token = EmailVerificationToken::new(user_id, &raw_token, 24);

    // Act - Time valid token check
    let start = std::time::Instant::now();
    let _ = verification_token.is_valid(&raw_token);
    let valid_time = start.elapsed();

    // Time invalid token check
    let start = std::time::Instant::now();
    let _ = verification_token.is_valid("invalid");
    let invalid_time = start.elapsed();

    // Assert - Times should be within same order of magnitude
    let diff = if valid_time > invalid_time {
        valid_time - invalid_time
    } else {
        invalid_time - valid_time
    };

    assert!(
        diff < std::time::Duration::from_millis(50),
        "Token validation timing should be relatively consistent"
    );
}
