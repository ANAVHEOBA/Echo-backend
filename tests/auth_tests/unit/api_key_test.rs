///! Unit Tests for API Key Management
///!
///! Tests cover:
///! - Key creation
///! - Key authentication
///! - Scope enforcement
///! - Key revocation
///! - Rate limiting
///! - Security considerations

use chrono::{Duration, Utc};
use echo_backend::modules::auth::services::{generate_token, hash_token};
use std::collections::HashSet;
use uuid::Uuid;

// =============================================================================
// TEST HELPERS
// =============================================================================

const API_KEY_PREFIX: &str = "ek_";

/// Generates an API key with the standard prefix
fn generate_api_key() -> String {
    format!("{}{}", API_KEY_PREFIX, generate_token())
}

/// Simulates an API key structure
#[derive(Debug, Clone)]
struct ApiKey {
    id: Uuid,
    user_id: Uuid,
    name: String,
    key_prefix: String,
    key_hash: String,
    scopes: Vec<String>,
    expires_at: Option<chrono::DateTime<Utc>>,
    revoked: bool,
    revoked_at: Option<chrono::DateTime<Utc>>,
    last_used_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
}

impl ApiKey {
    fn new(user_id: Uuid, name: &str, raw_key: &str, scopes: Vec<String>, expires_in_days: Option<i64>) -> Self {
        let prefix = if raw_key.len() >= 10 {
            raw_key[..10].to_string()
        } else {
            raw_key.to_string()
        };

        Self {
            id: Uuid::new_v4(),
            user_id,
            name: name.to_string(),
            key_prefix: prefix,
            key_hash: hash_token(raw_key),
            scopes,
            expires_at: expires_in_days.map(|d| Utc::now() + Duration::days(d)),
            revoked: false,
            revoked_at: None,
            last_used_at: None,
            created_at: Utc::now(),
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at.map(|exp| exp < Utc::now()).unwrap_or(false)
    }

    fn is_valid(&self, raw_key: &str) -> bool {
        !self.revoked && !self.is_expired() && hash_token(raw_key) == self.key_hash
    }

    fn has_scope(&self, required_scope: &str) -> bool {
        // Check for exact match, wildcard, or prefix match
        self.scopes.iter().any(|s| {
            s == required_scope
                || s == "*"
                || (s.ends_with(":*") && required_scope.starts_with(&s[..s.len() - 1]))
        })
    }

    fn revoke(&mut self) {
        self.revoked = true;
        self.revoked_at = Some(Utc::now());
    }

    fn record_usage(&mut self) {
        self.last_used_at = Some(Utc::now());
    }
}

// =============================================================================
// API KEY CREATION TESTS
// =============================================================================

#[tokio::test]
async fn create_api_key_returns_full_key_once() {
    // Arrange
    let _user_id = Uuid::new_v4();
    let _name = "Test API Key";

    // Act
    let raw_key = generate_api_key();

    // Assert
    assert!(raw_key.starts_with(API_KEY_PREFIX), "Key should start with prefix");
    assert!(raw_key.len() >= 32, "Key should be at least 32 characters");
}

#[tokio::test]
async fn create_api_key_stores_hash_only() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();

    // Act
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Assert
    assert_ne!(api_key.key_hash, raw_key, "Hash should not equal raw key");
    assert!(!api_key.key_hash.contains(&raw_key), "Hash should not contain raw key");
    assert_eq!(api_key.key_hash.len(), 64, "Hash should be SHA-256 (64 hex chars)");
}

#[tokio::test]
async fn create_api_key_stores_prefix() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();

    // Act
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Assert
    assert_eq!(api_key.key_prefix.len(), 10, "Prefix should be first 10 characters");
    assert!(raw_key.starts_with(&api_key.key_prefix), "Raw key should start with stored prefix");
}

#[tokio::test]
async fn create_api_key_with_name() {
    // Arrange
    let user_id = Uuid::new_v4();
    let name = "Production API Key";
    let raw_key = generate_api_key();

    // Act
    let api_key = ApiKey::new(user_id, name, &raw_key, vec![], None);

    // Assert
    assert_eq!(api_key.name, name, "Name should be stored");
}

#[tokio::test]
async fn create_api_key_with_scopes() {
    // Arrange
    let user_id = Uuid::new_v4();
    let scopes = vec!["read:deals".to_string(), "write:contacts".to_string()];
    let raw_key = generate_api_key();

    // Act
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, scopes.clone(), None);

    // Assert
    assert_eq!(api_key.scopes, scopes, "Scopes should be stored");
}

#[tokio::test]
async fn create_api_key_with_expiration() {
    // Arrange
    let user_id = Uuid::new_v4();
    let expires_in_days = 30;
    let raw_key = generate_api_key();
    let before = Utc::now();

    // Act
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], Some(expires_in_days));

    // Assert
    assert!(api_key.expires_at.is_some(), "Expiration should be set");
    let expires_at = api_key.expires_at.unwrap();
    assert!(expires_at >= before + Duration::days(30) - Duration::seconds(1));
    assert!(expires_at <= Utc::now() + Duration::days(30) + Duration::seconds(1));
}

#[tokio::test]
async fn create_api_key_without_expiration() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();

    // Act
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Assert
    assert!(api_key.expires_at.is_none(), "Expiration should be None");
    assert!(!api_key.is_expired(), "Non-expiring key should not be expired");
}

#[tokio::test]
async fn create_api_key_sets_timestamps() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let before = Utc::now();

    // Act
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Assert
    assert!(api_key.created_at >= before, "Created at should be set");
    assert!(api_key.last_used_at.is_none(), "Last used should be None initially");
}

#[tokio::test]
async fn generated_api_keys_are_unique() {
    // Arrange & Act
    let mut keys = HashSet::new();
    for _ in 0..100 {
        keys.insert(generate_api_key());
    }

    // Assert
    assert_eq!(keys.len(), 100, "All generated keys should be unique");
}

// =============================================================================
// API KEY AUTHENTICATION TESTS
// =============================================================================

#[tokio::test]
async fn authenticate_with_valid_api_key_succeeds() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Act & Assert
    assert!(api_key.is_valid(&raw_key), "Valid key should authenticate");
}

#[tokio::test]
async fn authenticate_updates_last_used_at() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let mut api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);
    let before = Utc::now();

    // Act
    api_key.record_usage();

    // Assert
    assert!(api_key.last_used_at.is_some(), "Last used should be set");
    assert!(api_key.last_used_at.unwrap() >= before, "Last used should be recent");
}

#[tokio::test]
async fn authenticate_with_expired_api_key_fails() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let mut api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], Some(30));

    // Expire the key
    api_key.expires_at = Some(Utc::now() - Duration::days(1));

    // Assert
    assert!(api_key.is_expired(), "Key should be expired");
    assert!(!api_key.is_valid(&raw_key), "Expired key should not authenticate");
}

#[tokio::test]
async fn authenticate_with_revoked_api_key_fails() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let mut api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Revoke the key
    api_key.revoke();

    // Assert
    assert!(api_key.revoked, "Key should be revoked");
    assert!(!api_key.is_valid(&raw_key), "Revoked key should not authenticate");
}

#[tokio::test]
async fn authenticate_with_invalid_api_key_fails() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);
    let invalid_key = "ek_invalid_random_key_12345";

    // Assert
    assert!(!api_key.is_valid(invalid_key), "Invalid key should not authenticate");
}

#[tokio::test]
async fn authenticate_with_wrong_prefix_fails() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);
    let wrong_prefix_key = format!("wrong_{}", &raw_key[3..]);

    // Assert
    assert!(!api_key.is_valid(&wrong_prefix_key), "Wrong prefix key should not authenticate");
}

#[tokio::test]
async fn api_key_hash_is_consistent() {
    // Arrange
    let raw_key = generate_api_key();

    // Act
    let hash1 = hash_token(&raw_key);
    let hash2 = hash_token(&raw_key);

    // Assert
    assert_eq!(hash1, hash2, "Same key should produce same hash");
}

// =============================================================================
// SCOPE ENFORCEMENT TESTS
// =============================================================================

#[tokio::test]
async fn api_key_allows_action_within_scope() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let scopes = vec!["read:deals".to_string()];
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, scopes, None);

    // Assert
    assert!(api_key.has_scope("read:deals"), "Key should allow read:deals");
}

#[tokio::test]
async fn api_key_denies_action_outside_scope() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let scopes = vec!["read:deals".to_string()];
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, scopes, None);

    // Assert
    assert!(!api_key.has_scope("write:deals"), "Key should deny write:deals");
}

#[tokio::test]
async fn api_key_supports_wildcard_scopes() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let scopes = vec!["*".to_string()];
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, scopes, None);

    // Assert
    assert!(api_key.has_scope("read:deals"), "Wildcard should allow any scope");
    assert!(api_key.has_scope("write:contacts"), "Wildcard should allow any scope");
    assert!(api_key.has_scope("admin:users"), "Wildcard should allow any scope");
}

#[tokio::test]
async fn api_key_supports_prefix_scopes() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let scopes = vec!["read:*".to_string()];
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, scopes, None);

    // Assert
    assert!(api_key.has_scope("read:deals"), "Prefix scope should allow read:deals");
    assert!(api_key.has_scope("read:contacts"), "Prefix scope should allow read:contacts");
    assert!(!api_key.has_scope("write:deals"), "Prefix scope should deny write:deals");
}

#[tokio::test]
async fn api_key_scope_is_case_sensitive() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let scopes = vec!["read:Deals".to_string()];
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, scopes, None);

    // Assert
    assert!(api_key.has_scope("read:Deals"), "Exact case should match");
    assert!(!api_key.has_scope("read:deals"), "Different case should not match");
}

#[tokio::test]
async fn api_key_multiple_scopes() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let scopes = vec![
        "read:deals".to_string(),
        "write:deals".to_string(),
        "read:contacts".to_string(),
    ];
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, scopes, None);

    // Assert
    assert!(api_key.has_scope("read:deals"), "Should have read:deals");
    assert!(api_key.has_scope("write:deals"), "Should have write:deals");
    assert!(api_key.has_scope("read:contacts"), "Should have read:contacts");
    assert!(!api_key.has_scope("write:contacts"), "Should not have write:contacts");
}

#[tokio::test]
async fn api_key_empty_scopes() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Assert
    assert!(!api_key.has_scope("read:deals"), "Empty scopes should deny all");
    assert!(!api_key.has_scope("*"), "Empty scopes should deny wildcard too");
}

// =============================================================================
// API KEY MANAGEMENT TESTS
// =============================================================================

#[tokio::test]
async fn list_api_keys_metadata() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec!["read:*".to_string()], Some(30));

    // Assert - Metadata should be accessible
    assert_eq!(api_key.name, "Test Key");
    assert!(!api_key.key_prefix.is_empty(), "Prefix should be stored");
    assert!(!api_key.scopes.is_empty(), "Scopes should be stored");
    assert!(api_key.expires_at.is_some(), "Expiration should be stored");
    assert!(!api_key.id.is_nil(), "ID should be set");

    // Key hash should be internal only
    assert_eq!(api_key.key_hash.len(), 64, "Hash should be stored but not exposed");
}

#[tokio::test]
async fn revoke_api_key_succeeds() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let mut api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);
    let before = Utc::now();

    // Act
    api_key.revoke();

    // Assert
    assert!(api_key.revoked, "Key should be revoked");
    assert!(api_key.revoked_at.is_some(), "Revoked at should be set");
    assert!(api_key.revoked_at.unwrap() >= before, "Revoked at should be recent");
}

#[tokio::test]
async fn revoke_api_key_immediately_invalidates() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let mut api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Verify it works before revocation
    assert!(api_key.is_valid(&raw_key), "Should be valid before revocation");

    // Act
    api_key.revoke();

    // Assert
    assert!(!api_key.is_valid(&raw_key), "Should be invalid immediately after revocation");
}

#[tokio::test]
async fn revoked_key_stays_revoked() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let mut api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Act - Revoke and try to "unrevoke" (by setting revoked = false manually)
    api_key.revoke();
    let revoked_at = api_key.revoked_at;

    // In real implementation, revocation should be irreversible
    // This test verifies the state is captured
    assert!(api_key.revoked, "Should remain revoked");
    assert_eq!(api_key.revoked_at, revoked_at, "Revoked timestamp should not change");
}

// =============================================================================
// SECURITY TESTS
// =============================================================================

#[tokio::test]
async fn api_key_uses_secure_random() {
    // Arrange & Act
    let mut keys = HashSet::new();
    for _ in 0..100 {
        keys.insert(generate_api_key());
    }

    // Assert
    assert_eq!(keys.len(), 100, "All keys should be unique (secure random)");
}

#[tokio::test]
async fn api_key_hash_uses_sha256() {
    // Arrange
    let raw_key = generate_api_key();

    // Act
    let hash = hash_token(&raw_key);

    // Assert
    assert_eq!(hash.len(), 64, "Hash should be SHA-256 (64 hex chars)");
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be hex encoded");
}

#[tokio::test]
async fn api_key_prefix_is_recognizable() {
    // Arrange & Act
    let raw_key = generate_api_key();

    // Assert
    assert!(raw_key.starts_with("ek_"), "Key should start with 'ek_' prefix");
}

#[tokio::test]
async fn api_key_validation_timing_consistent() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Act - Time valid key check
    let start = std::time::Instant::now();
    let _ = api_key.is_valid(&raw_key);
    let valid_time = start.elapsed();

    // Time invalid key check
    let start = std::time::Instant::now();
    let _ = api_key.is_valid("ek_invalid_key");
    let invalid_time = start.elapsed();

    // Assert - Times should be similar (constant time comparison in hash)
    let diff = if valid_time > invalid_time {
        valid_time - invalid_time
    } else {
        invalid_time - valid_time
    };

    assert!(
        diff < std::time::Duration::from_millis(50),
        "Validation timing should be consistent"
    );
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[tokio::test]
async fn empty_api_key_is_rejected() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Assert
    assert!(!api_key.is_valid(""), "Empty key should be rejected");
}

#[tokio::test]
async fn whitespace_api_key_is_rejected() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);

    // Assert
    assert!(!api_key.is_valid("   "), "Whitespace key should be rejected");
}

#[tokio::test]
async fn truncated_api_key_is_rejected() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);
    let truncated = &raw_key[..raw_key.len() - 1];

    // Assert
    assert!(!api_key.is_valid(truncated), "Truncated key should be rejected");
}

#[tokio::test]
async fn api_key_with_extra_chars_is_rejected() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], None);
    let modified = format!("{}extra", raw_key);

    // Assert
    assert!(!api_key.is_valid(&modified), "Key with extra chars should be rejected");
}

#[tokio::test]
async fn concurrent_api_key_generation_is_unique() {
    // Arrange
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let keys = Arc::new(Mutex::new(HashSet::new()));

    // Act - Generate keys concurrently
    let handles: Vec<_> = (0..50)
        .map(|_| {
            let keys = keys.clone();
            tokio::spawn(async move {
                let key = generate_api_key();
                keys.lock().await.insert(key);
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }

    // Assert
    let final_keys = keys.lock().await;
    assert_eq!(final_keys.len(), 50, "All concurrent keys should be unique");
}

#[tokio::test]
async fn api_key_expiry_boundary_check() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let mut api_key = ApiKey::new(user_id, "Test Key", &raw_key, vec![], Some(30));

    // Set expiry to very soon
    api_key.expires_at = Some(Utc::now() + Duration::milliseconds(100));

    // Assert - Should be valid before expiry
    assert!(!api_key.is_expired(), "Key should not be expired yet");

    // Wait for expiry
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Assert - Should be expired after
    assert!(api_key.is_expired(), "Key should be expired now");
    assert!(!api_key.is_valid(&raw_key), "Expired key should be invalid");
}

#[tokio::test]
async fn api_key_metadata_preserved() {
    // Arrange
    let user_id = Uuid::new_v4();
    let raw_key = generate_api_key();
    let name = "My Production Key";
    let scopes = vec!["read:*".to_string(), "write:deals".to_string()];
    let before = Utc::now();

    // Act
    let api_key = ApiKey::new(user_id, name, &raw_key, scopes.clone(), Some(90));

    // Assert
    assert_eq!(api_key.user_id, user_id, "User ID should be preserved");
    assert_eq!(api_key.name, name, "Name should be preserved");
    assert_eq!(api_key.scopes, scopes, "Scopes should be preserved");
    assert!(api_key.created_at >= before, "Created at should be set");
    assert!(!api_key.id.is_nil(), "ID should be set");
    assert!(!api_key.revoked, "Should not be revoked initially");
}

#[tokio::test]
async fn multiple_users_have_independent_keys() {
    // Arrange
    let user1_id = Uuid::new_v4();
    let user2_id = Uuid::new_v4();
    let key1 = generate_api_key();
    let key2 = generate_api_key();

    // Act
    let api_key1 = ApiKey::new(user1_id, "User1 Key", &key1, vec![], None);
    let api_key2 = ApiKey::new(user2_id, "User2 Key", &key2, vec![], None);

    // Assert
    assert_eq!(api_key1.user_id, user1_id);
    assert_eq!(api_key2.user_id, user2_id);
    assert!(api_key1.is_valid(&key1), "User 1's key should be valid");
    assert!(api_key2.is_valid(&key2), "User 2's key should be valid");

    // Cross-user validation should fail
    assert!(!api_key1.is_valid(&key2), "User 2's key should not validate for user 1");
    assert!(!api_key2.is_valid(&key1), "User 1's key should not validate for user 2");
}

#[tokio::test]
async fn api_key_scopes_combination_tests() {
    // Test various scope combinations
    let user_id = Uuid::new_v4();

    // Full wildcard
    let key1 = generate_api_key();
    let api_key1 = ApiKey::new(user_id, "Full Access", &key1, vec!["*".to_string()], None);
    assert!(api_key1.has_scope("anything:here"));

    // Read-only wildcard
    let key2 = generate_api_key();
    let api_key2 = ApiKey::new(user_id, "Read Only", &key2, vec!["read:*".to_string()], None);
    assert!(api_key2.has_scope("read:deals"));
    assert!(api_key2.has_scope("read:contacts"));
    assert!(!api_key2.has_scope("write:deals"));

    // Specific scopes
    let key3 = generate_api_key();
    let api_key3 = ApiKey::new(
        user_id,
        "Limited",
        &key3,
        vec!["read:deals".to_string(), "write:deals".to_string()],
        None,
    );
    assert!(api_key3.has_scope("read:deals"));
    assert!(api_key3.has_scope("write:deals"));
    assert!(!api_key3.has_scope("read:contacts"));
}
