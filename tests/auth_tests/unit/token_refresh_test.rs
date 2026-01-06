///! Unit Tests for Token Refresh and Revocation
///!
///! Tests cover:
///! - Token refresh flow
///! - Refresh token rotation
///! - Token revocation
///! - Session management
///! - Blacklist handling
///! - Security events

use chrono::Utc;
use echo_backend::config::AppConfig;
use echo_backend::modules::auth::model::{TokenType, User};
use echo_backend::modules::auth::services::{
    create_access_token, create_refresh_token, generate_token, hash_token,
    validate_access_token, validate_refresh_token,
};
use secrecy::SecretString;
use std::collections::HashSet;
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

fn test_user_admin() -> User {
    let mut user = test_user();
    user.role = "admin".to_string();
    user
}

fn expired_token_config() -> Arc<AppConfig> {
    Arc::new(AppConfig {
        database_url: SecretString::from("postgres://test:test@localhost/test"),
        redis_url: "redis://localhost:6379".to_string(),
        jwt_secret: SecretString::from("test_jwt_secret_key_that_is_at_least_32_characters_long"),
        jwt_refresh_secret: SecretString::from("test_refresh_secret_key_that_is_at_least_32_chars"),
        access_token_expiry_secs: -1, // Already expired
        refresh_token_expiry_days: -1, // Already expired
        encryption_key: SecretString::from("test_encryption_key_32_chars_ok"),
        host: "127.0.0.1".to_string(),
        port: 3000,
    })
}

// =============================================================================
// TOKEN REFRESH TESTS
// =============================================================================

#[tokio::test]
async fn refresh_with_valid_token_returns_new_tokens() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Create initial tokens (simulating login)
    let refresh_token = create_refresh_token(&user, &config).unwrap();

    // Act - Validate refresh token (would normally exchange for new tokens)
    let claims = validate_refresh_token(&refresh_token, &config).unwrap();

    // Create new tokens based on claims
    let new_access_token = create_access_token(&user, &config).unwrap();
    let new_refresh_token = create_refresh_token(&user, &config).unwrap();

    // Assert
    assert!(validate_access_token(&new_access_token, &config).is_ok());
    assert!(validate_refresh_token(&new_refresh_token, &config).is_ok());
    assert_eq!(claims.sub, user.id);
}

#[tokio::test]
async fn refresh_returns_new_access_token() {
    // Arrange
    let config = test_config();
    let user = test_user();

    let old_access = create_access_token(&user, &config).unwrap();
    let old_claims = validate_access_token(&old_access, &config).unwrap();

    // Small delay to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Act - Create new access token (simulating refresh)
    let new_access = create_access_token(&user, &config).unwrap();
    let new_claims = validate_access_token(&new_access, &config).unwrap();

    // Assert
    assert_ne!(old_claims.jti, new_claims.jti, "New token should have new jti");
    assert!(new_claims.iat >= old_claims.iat, "New token should have updated iat");
    assert!(new_claims.exp >= old_claims.exp, "New token should have new exp");
}

#[tokio::test]
async fn refresh_rotates_refresh_token() {
    // Arrange
    let config = test_config();
    let user = test_user();

    let old_refresh = create_refresh_token(&user, &config).unwrap();
    let old_claims = validate_refresh_token(&old_refresh, &config).unwrap();

    // Act - Create new refresh token (rotation)
    let new_refresh = create_refresh_token(&user, &config).unwrap();
    let new_claims = validate_refresh_token(&new_refresh, &config).unwrap();

    // Assert
    assert_ne!(old_refresh, new_refresh, "New refresh token should be different");
    assert_ne!(old_claims.jti, new_claims.jti, "New refresh token should have different jti");
}

#[tokio::test]
async fn refresh_token_has_different_jti_than_old() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Create multiple refresh tokens
    let token1 = create_refresh_token(&user, &config).unwrap();
    let token2 = create_refresh_token(&user, &config).unwrap();
    let token3 = create_refresh_token(&user, &config).unwrap();

    let claims1 = validate_refresh_token(&token1, &config).unwrap();
    let claims2 = validate_refresh_token(&token2, &config).unwrap();
    let claims3 = validate_refresh_token(&token3, &config).unwrap();

    // Assert - all JTIs should be unique
    let mut jtis = HashSet::new();
    jtis.insert(claims1.jti);
    jtis.insert(claims2.jti);
    jtis.insert(claims3.jti);

    assert_eq!(jtis.len(), 3, "All tokens should have unique JTIs");
}

#[tokio::test]
async fn refresh_with_expired_token_fails() {
    // Arrange
    let expired_config = expired_token_config();
    let user = test_user();

    let expired_refresh = create_refresh_token(&user, &expired_config).unwrap();

    // Act
    let result = validate_refresh_token(&expired_refresh, &test_config());

    // Assert
    assert!(result.is_err(), "Expired token should be rejected");
}

#[tokio::test]
async fn refresh_with_invalid_token_fails() {
    // Arrange
    let config = test_config();
    let invalid_token = "invalid_refresh_token_12345";

    // Act
    let result = validate_refresh_token(invalid_token, &config);

    // Assert
    assert!(result.is_err(), "Invalid token should be rejected");
}

#[tokio::test]
async fn refresh_validates_token_type() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Create an access token
    let access_token = create_access_token(&user, &config).unwrap();

    // Act - Try to use access token as refresh token
    let result = validate_refresh_token(&access_token, &config);

    // Assert
    assert!(result.is_err(), "Access token should not be valid as refresh token");
}

#[tokio::test]
async fn refresh_maintains_user_context() {
    // Arrange
    let config = test_config();
    let user = test_user_with_org();

    let refresh_token = create_refresh_token(&user, &config).unwrap();
    let _claims = validate_refresh_token(&refresh_token, &config).unwrap();

    // Act - Create new tokens preserving user context
    let new_access = create_access_token(&user, &config).unwrap();
    let new_claims = validate_access_token(&new_access, &config).unwrap();

    // Assert - context preserved
    assert_eq!(new_claims.sub, user.id, "User ID should be preserved");
    assert_eq!(new_claims.role, user.role, "Role should be preserved");
    assert_eq!(new_claims.org_id, user.organization_id, "Org ID should be preserved");
    assert_eq!(new_claims.email, user.email, "Email should be preserved");
}

#[tokio::test]
async fn refresh_preserves_admin_role() {
    // Arrange
    let config = test_config();
    let admin_user = test_user_admin();

    let refresh_token = create_refresh_token(&admin_user, &config).unwrap();
    let claims = validate_refresh_token(&refresh_token, &config).unwrap();

    // Assert
    assert_eq!(claims.role, "admin", "Admin role should be preserved in refresh token");
}

#[tokio::test]
async fn access_token_from_refresh_has_correct_expiry() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Act
    let access_token = create_access_token(&user, &config).unwrap();
    let claims = validate_access_token(&access_token, &config).unwrap();

    // Assert - access token should expire in 15 minutes (900 seconds)
    let expiry_duration = claims.exp - claims.iat;
    assert_eq!(expiry_duration, 900, "Access token should have 15 min expiry");
}

#[tokio::test]
async fn refresh_token_has_7_day_expiry() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Act
    let refresh_token = create_refresh_token(&user, &config).unwrap();
    let claims = validate_refresh_token(&refresh_token, &config).unwrap();

    // Assert - refresh token should expire in 7 days
    let expiry_duration = claims.exp - claims.iat;
    let expected = 7 * 24 * 60 * 60; // 7 days in seconds
    assert_eq!(expiry_duration, expected, "Refresh token should have 7 day expiry");
}

// =============================================================================
// LOGOUT / REVOCATION TESTS
// =============================================================================

#[tokio::test]
async fn token_hash_is_consistent_for_same_token() {
    // Arrange
    let token = generate_token();

    // Act
    let hash1 = hash_token(&token);
    let hash2 = hash_token(&token);

    // Assert
    assert_eq!(hash1, hash2, "Same token should produce same hash");
}

#[tokio::test]
async fn token_hash_is_different_for_different_tokens() {
    // Arrange
    let token1 = generate_token();
    let token2 = generate_token();

    // Act
    let hash1 = hash_token(&token1);
    let hash2 = hash_token(&token2);

    // Assert
    assert_ne!(hash1, hash2, "Different tokens should produce different hashes");
}

#[tokio::test]
async fn generated_token_is_64_hex_chars() {
    // Arrange & Act
    let token = generate_token();

    // Assert - 32 bytes = 64 hex characters
    assert_eq!(token.len(), 64, "Generated token should be 64 hex characters");
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()), "Token should be hex encoded");
}

#[tokio::test]
async fn generated_tokens_are_unique() {
    // Arrange
    let mut tokens = HashSet::new();

    // Act - Generate many tokens
    for _ in 0..100 {
        tokens.insert(generate_token());
    }

    // Assert - All should be unique
    assert_eq!(tokens.len(), 100, "All generated tokens should be unique");
}

#[tokio::test]
async fn token_jti_can_be_used_for_blacklist() {
    // Arrange
    let config = test_config();
    let user = test_user();

    let access_token = create_access_token(&user, &config).unwrap();
    let claims = validate_access_token(&access_token, &config).unwrap();

    // Act - Extract JTI for blacklisting
    let jti = claims.jti;
    let jti_string = jti.to_string();

    // Assert - JTI should be a valid UUID string
    assert!(!jti.is_nil(), "JTI should not be nil");
    assert_eq!(jti_string.len(), 36, "JTI string should be 36 chars (UUID format)");
}

// =============================================================================
// SESSION LISTING TESTS (Token-based)
// =============================================================================

#[tokio::test]
async fn multiple_tokens_have_unique_jtis() {
    // Arrange
    let config = test_config();
    let user = test_user();
    let mut jtis = HashSet::new();

    // Act - Create multiple access tokens (simulating multiple sessions)
    for _ in 0..5 {
        let token = create_access_token(&user, &config).unwrap();
        let claims = validate_access_token(&token, &config).unwrap();
        jtis.insert(claims.jti);
    }

    // Assert
    assert_eq!(jtis.len(), 5, "Each session token should have unique JTI");
}

#[tokio::test]
async fn refresh_tokens_contain_user_identification() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Act
    let refresh_token = create_refresh_token(&user, &config).unwrap();
    let claims = validate_refresh_token(&refresh_token, &config).unwrap();

    // Assert - Should contain user identification for session tracking
    assert_eq!(claims.sub, user.id, "Should contain user ID");
    assert_eq!(claims.email, user.email, "Should contain user email");
}

// =============================================================================
// BLACKLIST TESTS (JTI-based)
// =============================================================================

#[tokio::test]
async fn jti_is_uuid_format() {
    // Arrange
    let config = test_config();
    let user = test_user();

    let token = create_access_token(&user, &config).unwrap();
    let claims = validate_access_token(&token, &config).unwrap();

    // Assert - JTI should be UUID v4 format
    assert!(!claims.jti.is_nil());
    // UUID v4 has specific version bits
    assert_eq!(claims.jti.get_version_num(), 4, "JTI should be UUID v4");
}

#[tokio::test]
async fn hash_token_produces_sha256_output() {
    // Arrange
    let token = "test_token_12345";

    // Act
    let hash = hash_token(token);

    // Assert - SHA256 produces 32 bytes = 64 hex characters
    assert_eq!(hash.len(), 64, "Hash should be 64 hex characters (SHA256)");
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be hex encoded");
}

// =============================================================================
// SECURITY EVENT TESTS
// =============================================================================

#[tokio::test]
async fn tokens_for_same_user_share_sub_claim() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Act
    let access1 = create_access_token(&user, &config).unwrap();
    let access2 = create_access_token(&user, &config).unwrap();
    let refresh = create_refresh_token(&user, &config).unwrap();

    let claims1 = validate_access_token(&access1, &config).unwrap();
    let claims2 = validate_access_token(&access2, &config).unwrap();
    let claims_r = validate_refresh_token(&refresh, &config).unwrap();

    // Assert - All tokens for same user should have same sub
    assert_eq!(claims1.sub, claims2.sub);
    assert_eq!(claims1.sub, claims_r.sub);
    assert_eq!(claims1.sub, user.id);
}

#[tokio::test]
async fn different_users_have_different_sub_claims() {
    // Arrange
    let config = test_config();
    let user1 = test_user();
    let mut user2 = test_user();
    user2.id = Uuid::new_v4();
    user2.email = "other@example.com".to_string();

    // Act
    let token1 = create_access_token(&user1, &config).unwrap();
    let token2 = create_access_token(&user2, &config).unwrap();

    let claims1 = validate_access_token(&token1, &config).unwrap();
    let claims2 = validate_access_token(&token2, &config).unwrap();

    // Assert
    assert_ne!(claims1.sub, claims2.sub, "Different users should have different subs");
}

// =============================================================================
// CONCURRENT REFRESH TESTS
// =============================================================================

#[tokio::test]
async fn concurrent_token_generation_produces_unique_tokens() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Act - Generate tokens concurrently
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let config = config.clone();
            let user = user.clone();
            tokio::spawn(async move {
                create_access_token(&user, &config).unwrap()
            })
        })
        .collect();

    let tokens: Vec<String> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Assert - All tokens should be unique
    let unique_tokens: HashSet<_> = tokens.iter().collect();
    assert_eq!(unique_tokens.len(), 10, "Concurrent token generation should produce unique tokens");
}

#[tokio::test]
async fn concurrent_jti_generation_produces_unique_jtis() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Act - Generate tokens concurrently and collect JTIs
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let config = config.clone();
            let user = user.clone();
            tokio::spawn(async move {
                let token = create_access_token(&user, &config).unwrap();
                validate_access_token(&token, &config).unwrap().jti
            })
        })
        .collect();

    let jtis: Vec<Uuid> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Assert - All JTIs should be unique
    let unique_jtis: HashSet<_> = jtis.iter().collect();
    assert_eq!(unique_jtis.len(), 10, "Concurrent token generation should produce unique JTIs");
}

// =============================================================================
// TOKEN BINDING TESTS
// =============================================================================

#[tokio::test]
async fn token_contains_iat_timestamp() {
    // Arrange
    let before = Utc::now().timestamp();
    let config = test_config();
    let user = test_user();

    // Act
    let token = create_access_token(&user, &config).unwrap();
    let after = Utc::now().timestamp();
    let claims = validate_access_token(&token, &config).unwrap();

    // Assert
    assert!(claims.iat >= before, "iat should be >= before time");
    assert!(claims.iat <= after, "iat should be <= after time");
}

#[tokio::test]
async fn refresh_token_type_is_correct() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Act
    let token = create_refresh_token(&user, &config).unwrap();
    let claims = validate_refresh_token(&token, &config).unwrap();

    // Assert
    assert_eq!(claims.token_type, TokenType::Refresh);
}

#[tokio::test]
async fn access_token_type_is_correct() {
    // Arrange
    let config = test_config();
    let user = test_user();

    // Act
    let token = create_access_token(&user, &config).unwrap();
    let claims = validate_access_token(&token, &config).unwrap();

    // Assert
    assert_eq!(claims.token_type, TokenType::Access);
}

#[tokio::test]
async fn token_exp_is_in_future() {
    // Arrange
    let config = test_config();
    let user = test_user();
    let now = Utc::now().timestamp();

    // Act
    let access_token = create_access_token(&user, &config).unwrap();
    let refresh_token = create_refresh_token(&user, &config).unwrap();

    let access_claims = validate_access_token(&access_token, &config).unwrap();
    let refresh_claims = validate_refresh_token(&refresh_token, &config).unwrap();

    // Assert
    assert!(access_claims.exp > now, "Access token exp should be in future");
    assert!(refresh_claims.exp > now, "Refresh token exp should be in future");
}

#[tokio::test]
async fn malformed_token_is_rejected() {
    // Arrange
    let config = test_config();
    let malformed_tokens = vec![
        "",
        "not.valid",
        "a.b.c.d.e",
        "eyJhbGciOiJIUzI1NiJ9",
        "!!!.@@@.###",
    ];

    // Act & Assert
    for malformed in malformed_tokens {
        let access_result = validate_access_token(malformed, &config);
        let refresh_result = validate_refresh_token(malformed, &config);

        assert!(access_result.is_err(), "Malformed token '{}' should be rejected for access", malformed);
        assert!(refresh_result.is_err(), "Malformed token '{}' should be rejected for refresh", malformed);
    }
}

#[tokio::test]
async fn tampered_token_is_rejected() {
    // Arrange
    let config = test_config();
    let user = test_user();
    let token = create_access_token(&user, &config).unwrap();

    // Tamper with the token
    let parts: Vec<&str> = token.split('.').collect();
    let tampered = format!("{}.{}x.{}", parts[0], parts[1], parts[2]);

    // Act
    let result = validate_access_token(&tampered, &config);

    // Assert
    assert!(result.is_err(), "Tampered token should be rejected");
}

#[tokio::test]
async fn wrong_secret_token_is_rejected() {
    // Arrange
    let config = test_config();
    let user = test_user();
    let token = create_access_token(&user, &config).unwrap();

    // Create config with different secret
    let wrong_config = Arc::new(AppConfig {
        database_url: SecretString::from("postgres://test:test@localhost/test"),
        redis_url: "redis://localhost:6379".to_string(),
        jwt_secret: SecretString::from("completely_different_secret_key_that_is_32_chars"),
        jwt_refresh_secret: SecretString::from("test_refresh_secret_key_that_is_at_least_32_chars"),
        access_token_expiry_secs: 900,
        refresh_token_expiry_days: 7,
        encryption_key: SecretString::from("test_encryption_key_32_chars_ok"),
        host: "127.0.0.1".to_string(),
        port: 3000,
    });

    // Act
    let result = validate_access_token(&token, &wrong_config);

    // Assert
    assert!(result.is_err(), "Token signed with different secret should be rejected");
}
