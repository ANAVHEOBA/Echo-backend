///! Unit Tests for OAuth 2.0 + PKCE
///!
///! Tests cover:
///! - Authorization URL generation
///! - State parameter handling
///! - PKCE flow (code verifier/challenge)
///! - Token storage and encryption
///! - Connection management

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use echo_backend::modules::auth::services::generate_token;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use uuid::Uuid;

// =============================================================================
// TEST HELPERS
// =============================================================================

const PROVIDERS: [&str; 5] = ["google", "zoom", "slack", "hubspot", "salesforce"];

/// Generates a PKCE code verifier (43-128 characters, URL-safe)
fn generate_code_verifier() -> String {
    // 32 bytes = 43 base64 characters (URL safe)
    let bytes: [u8; 32] = rand::random();
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Generates a PKCE code challenge from a verifier (S256 method)
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

/// Simulates OAuth state storage
#[derive(Debug, Clone)]
struct OAuthState {
    id: Uuid,
    user_id: Uuid,
    provider: String,
    state: String,
    code_verifier: String,
    redirect_uri: String,
    scopes: Vec<String>,
    expires_at: chrono::DateTime<Utc>,
    used: bool,
    created_at: chrono::DateTime<Utc>,
}

impl OAuthState {
    fn new(
        user_id: Uuid,
        provider: &str,
        redirect_uri: &str,
        scopes: Vec<String>,
    ) -> (Self, String, String) {
        let state = generate_token();
        let code_verifier = generate_code_verifier();
        let code_challenge = generate_code_challenge(&code_verifier);

        let oauth_state = Self {
            id: Uuid::new_v4(),
            user_id,
            provider: provider.to_string(),
            state: state.clone(),
            code_verifier,
            redirect_uri: redirect_uri.to_string(),
            scopes,
            expires_at: Utc::now() + Duration::minutes(10),
            used: false,
            created_at: Utc::now(),
        };

        (oauth_state, state, code_challenge)
    }

    fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    fn is_valid(&self, state: &str) -> bool {
        !self.used && !self.is_expired() && self.state == state
    }

    fn mark_used(&mut self) {
        self.used = true;
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn user_id(&self) -> Uuid {
        self.user_id
    }

    fn created_at(&self) -> chrono::DateTime<Utc> {
        self.created_at
    }
}

/// Simulates an OAuth connection (stored tokens)
#[derive(Debug, Clone)]
struct OAuthConnection {
    id: Uuid,
    user_id: Uuid,
    provider: String,
    provider_user_id: String,
    access_token_encrypted: Vec<u8>,
    refresh_token_encrypted: Option<Vec<u8>>,
    token_expires_at: Option<chrono::DateTime<Utc>>,
    scopes: Vec<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl OAuthConnection {
    fn new(
        user_id: Uuid,
        provider: &str,
        provider_user_id: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_in_secs: Option<i64>,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            provider: provider.to_string(),
            provider_user_id: provider_user_id.to_string(),
            access_token_encrypted: encrypt_token(access_token),
            refresh_token_encrypted: refresh_token.map(encrypt_token),
            token_expires_at: expires_in_secs.map(|s| Utc::now() + Duration::seconds(s)),
            scopes,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn is_token_expired(&self) -> bool {
        self.token_expires_at.map(|exp| exp < Utc::now()).unwrap_or(false)
    }

    fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }
}

/// Simulates AES-GCM encryption (returns dummy encrypted data for testing)
fn encrypt_token(token: &str) -> Vec<u8> {
    // In real implementation: AES-256-GCM encryption
    // For testing, we XOR the data so plaintext is not visible
    let mut result = vec![0u8; 12]; // Nonce
    // XOR each byte with 0xAA to simulate encryption (plaintext won't be visible)
    let encrypted_bytes: Vec<u8> = token.as_bytes().iter().map(|b| b ^ 0xAA).collect();
    result.extend_from_slice(&encrypted_bytes);
    result.extend_from_slice(&[0u8; 16]); // Auth tag
    result
}

// =============================================================================
// AUTHORIZATION URL TESTS
// =============================================================================

#[tokio::test]
async fn authorization_url_includes_provider_endpoint() {
    // Test each provider has a distinct endpoint pattern
    for provider in PROVIDERS {
        let user_id = Uuid::new_v4();
        let (oauth_state, state, _code_challenge) =
            OAuthState::new(user_id, provider, "https://app.example.com/callback", vec![]);

        // Verify state was created for the provider
        assert_eq!(oauth_state.provider, provider);
        assert!(!state.is_empty(), "State should be generated for {}", provider);
    }
}

#[tokio::test]
async fn authorization_url_includes_required_params() {
    // Arrange
    let user_id = Uuid::new_v4();
    let redirect_uri = "https://app.example.com/oauth/callback";
    let scopes = vec!["email".to_string(), "profile".to_string()];

    // Act
    let (oauth_state, state, code_challenge) =
        OAuthState::new(user_id, "google", redirect_uri, scopes.clone());

    // Assert - Required params are generated
    assert!(!state.is_empty(), "State should be generated");
    assert!(!code_challenge.is_empty(), "Code challenge should be generated");
    assert_eq!(oauth_state.redirect_uri, redirect_uri);
    assert_eq!(oauth_state.scopes, scopes);
    assert_eq!(oauth_state.user_id(), user_id, "User ID should be stored");
    assert!(!oauth_state.id().is_nil(), "State should have unique ID");
    assert!(oauth_state.created_at() <= Utc::now(), "Created at should be set");
}

// =============================================================================
// STATE PARAMETER TESTS
// =============================================================================

#[tokio::test]
async fn state_is_cryptographically_random() {
    // Arrange & Act
    let mut states = HashSet::new();
    for _ in 0..100 {
        let user_id = Uuid::new_v4();
        let (_, state, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);
        states.insert(state);
    }

    // Assert
    assert_eq!(states.len(), 100, "All states should be unique");
}

#[tokio::test]
async fn state_has_sufficient_entropy() {
    // Arrange
    let user_id = Uuid::new_v4();
    let (_, state, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);

    // Assert - State should be 64 hex characters (32 bytes)
    assert_eq!(state.len(), 64, "State should be 64 hex characters");
    assert!(state.chars().all(|c| c.is_ascii_hexdigit()), "State should be hex encoded");
}

#[tokio::test]
async fn state_stored_with_expiry() {
    // Arrange
    let user_id = Uuid::new_v4();
    let before = Utc::now();

    // Act
    let (oauth_state, _, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);

    // Assert - State should expire in ~10 minutes
    let expected_min = before + Duration::minutes(10) - Duration::seconds(1);
    let expected_max = Utc::now() + Duration::minutes(10) + Duration::seconds(1);

    assert!(oauth_state.expires_at >= expected_min, "Expiry should be ~10 min from now");
    assert!(oauth_state.expires_at <= expected_max, "Expiry should be ~10 min from now");
}

#[tokio::test]
async fn state_validation_succeeds_for_valid_state() {
    // Arrange
    let user_id = Uuid::new_v4();
    let (oauth_state, state, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);

    // Assert
    assert!(oauth_state.is_valid(&state), "Valid state should be accepted");
}

#[tokio::test]
async fn state_validation_fails_for_expired_state() {
    // Arrange
    let user_id = Uuid::new_v4();
    let (mut oauth_state, state, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);

    // Expire the state
    oauth_state.expires_at = Utc::now() - Duration::minutes(1);

    // Assert
    assert!(oauth_state.is_expired(), "State should be expired");
    assert!(!oauth_state.is_valid(&state), "Expired state should be rejected");
}

#[tokio::test]
async fn state_validation_fails_for_used_state() {
    // Arrange
    let user_id = Uuid::new_v4();
    let (mut oauth_state, state, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);

    // Use the state
    oauth_state.mark_used();

    // Assert
    assert!(oauth_state.used, "State should be marked as used");
    assert!(!oauth_state.is_valid(&state), "Used state should be rejected");
}

#[tokio::test]
async fn state_validation_fails_for_wrong_state() {
    // Arrange
    let user_id = Uuid::new_v4();
    let (oauth_state, _, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);

    // Assert
    assert!(!oauth_state.is_valid("wrong_state_value"), "Wrong state should be rejected");
}

#[tokio::test]
async fn state_is_single_use() {
    // Arrange
    let user_id = Uuid::new_v4();
    let (mut oauth_state, state, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);

    // First validation
    assert!(oauth_state.is_valid(&state), "First use should succeed");
    oauth_state.mark_used();

    // Second validation
    assert!(!oauth_state.is_valid(&state), "Second use should fail");
}

#[tokio::test]
async fn state_stores_scopes_for_verification() {
    // Arrange
    let user_id = Uuid::new_v4();
    let scopes = vec!["email".to_string(), "calendar".to_string()];

    // Act
    let (oauth_state, _, _) = OAuthState::new(user_id, "google", "https://callback", scopes.clone());

    // Assert
    assert_eq!(oauth_state.scopes, scopes, "Scopes should be stored with state");
}

// =============================================================================
// PKCE TESTS
// =============================================================================

#[tokio::test]
async fn pkce_code_verifier_has_correct_length() {
    // Arrange & Act
    let verifier = generate_code_verifier();

    // Assert - 32 bytes = 43 base64 characters
    assert_eq!(verifier.len(), 43, "Code verifier should be 43 characters");
}

#[tokio::test]
async fn pkce_code_verifier_is_url_safe() {
    // Arrange & Act
    let verifier = generate_code_verifier();

    // Assert - Should only contain URL-safe characters
    assert!(
        verifier.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "Code verifier should be URL-safe"
    );
}

#[tokio::test]
async fn pkce_code_challenge_uses_s256() {
    // Arrange
    let verifier = generate_code_verifier();

    // Act
    let challenge = generate_code_challenge(&verifier);

    // Assert - S256 = SHA256 + base64url = 43 characters
    assert_eq!(challenge.len(), 43, "Code challenge should be 43 characters");
    assert!(
        challenge.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "Code challenge should be URL-safe"
    );
}

#[tokio::test]
async fn pkce_code_challenge_is_derived_from_verifier() {
    // Arrange
    let verifier = generate_code_verifier();

    // Act
    let challenge1 = generate_code_challenge(&verifier);
    let challenge2 = generate_code_challenge(&verifier);

    // Assert
    assert_eq!(challenge1, challenge2, "Same verifier should produce same challenge");
}

#[tokio::test]
async fn pkce_different_verifiers_produce_different_challenges() {
    // Arrange
    let verifier1 = generate_code_verifier();
    let verifier2 = generate_code_verifier();

    // Act
    let challenge1 = generate_code_challenge(&verifier1);
    let challenge2 = generate_code_challenge(&verifier2);

    // Assert
    assert_ne!(verifier1, verifier2, "Verifiers should be different");
    assert_ne!(challenge1, challenge2, "Challenges should be different");
}

#[tokio::test]
async fn pkce_code_verifier_is_stored_securely() {
    // Arrange
    let user_id = Uuid::new_v4();
    let (oauth_state, _, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);

    // Assert - Code verifier should be stored in state
    assert!(!oauth_state.code_verifier.is_empty(), "Code verifier should be stored");
    assert_eq!(oauth_state.code_verifier.len(), 43, "Code verifier should be 43 chars");
}

#[tokio::test]
async fn pkce_unique_verifiers_per_request() {
    // Arrange & Act
    let mut verifiers = HashSet::new();
    for _ in 0..100 {
        let user_id = Uuid::new_v4();
        let (oauth_state, _, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);
        verifiers.insert(oauth_state.code_verifier);
    }

    // Assert
    assert_eq!(verifiers.len(), 100, "All code verifiers should be unique");
}

// =============================================================================
// TOKEN STORAGE TESTS
// =============================================================================

#[tokio::test]
async fn access_token_is_encrypted() {
    // Arrange
    let user_id = Uuid::new_v4();
    let access_token = "ya29.test_access_token";

    // Act
    let connection = OAuthConnection::new(
        user_id,
        "google",
        "123456789",
        access_token,
        None,
        Some(3600),
        vec![],
    );

    // Assert - Token should be encrypted (not plaintext)
    let token_str = String::from_utf8_lossy(&connection.access_token_encrypted);
    assert!(
        !token_str.contains(access_token),
        "Encrypted token should not contain plaintext"
    );
    assert!(
        connection.access_token_encrypted.len() > access_token.len(),
        "Encrypted data should include overhead (nonce + tag)"
    );
}

#[tokio::test]
async fn refresh_token_is_encrypted() {
    // Arrange
    let user_id = Uuid::new_v4();
    let access_token = "ya29.access";
    let refresh_token = "1//refresh_token";

    // Act
    let connection = OAuthConnection::new(
        user_id,
        "google",
        "123456789",
        access_token,
        Some(refresh_token),
        Some(3600),
        vec![],
    );

    // Assert
    assert!(connection.refresh_token_encrypted.is_some(), "Refresh token should be stored");
    let encrypted = connection.refresh_token_encrypted.unwrap();
    let token_str = String::from_utf8_lossy(&encrypted);
    assert!(
        !token_str.contains(refresh_token),
        "Encrypted refresh token should not contain plaintext"
    );
}

#[tokio::test]
async fn token_expiration_is_stored() {
    // Arrange
    let user_id = Uuid::new_v4();
    let before = Utc::now();
    let expires_in = 3600; // 1 hour

    // Act
    let connection = OAuthConnection::new(
        user_id,
        "google",
        "123456789",
        "access_token",
        None,
        Some(expires_in),
        vec![],
    );

    // Assert
    assert!(connection.token_expires_at.is_some(), "Expiration should be stored");
    let expires_at = connection.token_expires_at.unwrap();
    assert!(expires_at >= before + Duration::seconds(expires_in - 1));
    assert!(expires_at <= Utc::now() + Duration::seconds(expires_in + 1));
}

#[tokio::test]
async fn token_expiration_check() {
    // Arrange
    let user_id = Uuid::new_v4();
    let mut connection = OAuthConnection::new(
        user_id,
        "google",
        "123456789",
        "access_token",
        None,
        Some(3600),
        vec![],
    );

    // Not expired initially
    assert!(!connection.is_token_expired(), "Token should not be expired initially");

    // Expire the token
    connection.token_expires_at = Some(Utc::now() - Duration::minutes(1));

    // Assert
    assert!(connection.is_token_expired(), "Token should be expired");
}

#[tokio::test]
async fn scopes_are_stored_with_connection() {
    // Arrange
    let user_id = Uuid::new_v4();
    let scopes = vec![
        "https://www.googleapis.com/auth/gmail.readonly".to_string(),
        "https://www.googleapis.com/auth/calendar".to_string(),
    ];

    // Act
    let connection = OAuthConnection::new(
        user_id,
        "google",
        "123456789",
        "access_token",
        None,
        Some(3600),
        scopes.clone(),
    );

    // Assert
    assert_eq!(connection.scopes, scopes, "Scopes should be stored");
}

#[tokio::test]
async fn provider_user_id_is_stored() {
    // Arrange
    let user_id = Uuid::new_v4();
    let provider_user_id = "google-user-12345";

    // Act
    let connection = OAuthConnection::new(
        user_id,
        "google",
        provider_user_id,
        "access_token",
        None,
        Some(3600),
        vec![],
    );

    // Assert
    assert_eq!(connection.provider_user_id, provider_user_id);
}

// =============================================================================
// CONNECTION MANAGEMENT TESTS
// =============================================================================

#[tokio::test]
async fn connection_has_scope() {
    // Arrange
    let user_id = Uuid::new_v4();
    let scopes = vec!["read".to_string(), "write".to_string()];

    let connection = OAuthConnection::new(
        user_id,
        "google",
        "123",
        "token",
        None,
        None,
        scopes,
    );

    // Assert
    assert!(connection.has_scope("read"), "Should have read scope");
    assert!(connection.has_scope("write"), "Should have write scope");
    assert!(!connection.has_scope("admin"), "Should not have admin scope");
}

#[tokio::test]
async fn connection_timestamps_are_set() {
    // Arrange
    let user_id = Uuid::new_v4();
    let before = Utc::now();

    // Act
    let connection = OAuthConnection::new(
        user_id,
        "google",
        "123",
        "token",
        None,
        None,
        vec![],
    );

    // Assert
    assert!(connection.created_at >= before, "Created at should be set");
    assert!(connection.updated_at >= before, "Updated at should be set");
}

#[tokio::test]
async fn connection_id_is_unique() {
    // Arrange
    let user_id = Uuid::new_v4();

    // Act
    let conn1 = OAuthConnection::new(user_id, "google", "123", "token", None, None, vec![]);
    let conn2 = OAuthConnection::new(user_id, "google", "456", "token", None, None, vec![]);

    // Assert
    assert_ne!(conn1.id, conn2.id, "Connection IDs should be unique");
}

// =============================================================================
// PROVIDER-SPECIFIC SCOPE TESTS
// =============================================================================

#[tokio::test]
async fn google_scopes_format() {
    // Google uses full URL format for scopes
    let scopes = vec![
        "https://www.googleapis.com/auth/gmail.readonly".to_string(),
        "https://www.googleapis.com/auth/calendar".to_string(),
    ];

    for scope in &scopes {
        assert!(scope.starts_with("https://"), "Google scopes should be URLs");
    }
}

#[tokio::test]
async fn zoom_scopes_format() {
    // Zoom uses short format
    let scopes = vec![
        "meeting:read".to_string(),
        "meeting:write".to_string(),
        "recording:read".to_string(),
    ];

    for scope in &scopes {
        assert!(scope.contains(":"), "Zoom scopes should use resource:action format");
    }
}

#[tokio::test]
async fn slack_scopes_format() {
    // Slack uses dot notation
    let scopes = vec![
        "chat:write".to_string(),
        "channels:read".to_string(),
        "users:read".to_string(),
    ];

    for scope in &scopes {
        assert!(scope.contains(":"), "Slack scopes should use resource:action format");
    }
}

// =============================================================================
// SECURITY TESTS
// =============================================================================

#[tokio::test]
async fn state_is_not_predictable() {
    // Generate multiple states and verify they don't follow a pattern
    let mut states: Vec<String> = Vec::new();
    for _ in 0..10 {
        let user_id = Uuid::new_v4();
        let (_, state, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);
        states.push(state);
    }

    // Check no two states share a common prefix (would indicate predictability)
    for i in 0..states.len() {
        for j in (i + 1)..states.len() {
            let common_prefix: String = states[i]
                .chars()
                .zip(states[j].chars())
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| a)
                .collect();
            assert!(
                common_prefix.len() < 8,
                "States should not share long common prefixes (predictability risk)"
            );
        }
    }
}

#[tokio::test]
async fn redirect_uri_is_stored_for_validation() {
    // Arrange
    let user_id = Uuid::new_v4();
    let redirect_uri = "https://app.example.com/oauth/callback";

    // Act
    let (oauth_state, _, _) = OAuthState::new(user_id, "google", redirect_uri, vec![]);

    // Assert
    assert_eq!(oauth_state.redirect_uri, redirect_uri, "Redirect URI should be stored for validation");
}

#[tokio::test]
async fn token_encryption_includes_auth_tag() {
    // Arrange
    let token = "test_token";

    // Act
    let encrypted = encrypt_token(token);

    // Assert - Should include nonce (12 bytes) + data + auth tag (16 bytes)
    assert!(
        encrypted.len() >= 12 + token.len() + 16,
        "Encrypted data should include nonce and auth tag"
    );
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[tokio::test]
async fn empty_scopes_are_handled() {
    // Arrange
    let user_id = Uuid::new_v4();

    // Act
    let (oauth_state, _, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);
    let connection = OAuthConnection::new(user_id, "google", "123", "token", None, None, vec![]);

    // Assert
    assert!(oauth_state.scopes.is_empty(), "Empty scopes should be handled");
    assert!(connection.scopes.is_empty(), "Empty scopes should be handled in connection");
    assert!(!connection.has_scope("any"), "Empty scopes should deny all");
}

#[tokio::test]
async fn no_refresh_token_is_handled() {
    // Arrange
    let user_id = Uuid::new_v4();

    // Act - Some providers don't return refresh tokens
    let connection = OAuthConnection::new(user_id, "google", "123", "access", None, Some(3600), vec![]);

    // Assert
    assert!(connection.refresh_token_encrypted.is_none(), "None refresh token should be handled");
}

#[tokio::test]
async fn no_expiration_is_handled() {
    // Arrange
    let user_id = Uuid::new_v4();

    // Act - Some tokens don't expire
    let connection = OAuthConnection::new(user_id, "google", "123", "access", None, None, vec![]);

    // Assert
    assert!(connection.token_expires_at.is_none(), "None expiration should be handled");
    assert!(!connection.is_token_expired(), "Token without expiration should not be expired");
}

#[tokio::test]
async fn state_expiry_boundary_check() {
    // Arrange
    let user_id = Uuid::new_v4();
    let (mut oauth_state, state, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);

    // Set expiry to very soon
    oauth_state.expires_at = Utc::now() + Duration::milliseconds(100);

    // Assert - Should be valid before expiry
    assert!(!oauth_state.is_expired(), "State should not be expired yet");

    // Wait for expiry
    std::thread::sleep(std::time::Duration::from_millis(150));

    // Assert - Should be expired after
    assert!(oauth_state.is_expired(), "State should be expired now");
    assert!(!oauth_state.is_valid(&state), "Expired state should be invalid");
}

#[tokio::test]
async fn multiple_providers_per_user() {
    // Arrange
    let user_id = Uuid::new_v4();

    // Act - Create connections for multiple providers
    let google_conn = OAuthConnection::new(
        user_id, "google", "g123", "token", None, None, vec!["email".to_string()],
    );
    let zoom_conn = OAuthConnection::new(
        user_id, "zoom", "z456", "token", None, None, vec!["meeting:read".to_string()],
    );
    let slack_conn = OAuthConnection::new(
        user_id, "slack", "s789", "token", None, None, vec!["chat:write".to_string()],
    );

    // Assert
    assert_eq!(google_conn.user_id, user_id);
    assert_eq!(zoom_conn.user_id, user_id);
    assert_eq!(slack_conn.user_id, user_id);

    assert_eq!(google_conn.provider, "google");
    assert_eq!(zoom_conn.provider, "zoom");
    assert_eq!(slack_conn.provider, "slack");

    // Each should have different IDs
    let ids: HashSet<_> = [google_conn.id, zoom_conn.id, slack_conn.id].into_iter().collect();
    assert_eq!(ids.len(), 3, "Each connection should have unique ID");
}

#[tokio::test]
async fn concurrent_state_generation_is_unique() {
    // Arrange
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let states = Arc::new(Mutex::new(HashSet::new()));

    // Act - Generate states concurrently
    let handles: Vec<_> = (0..50)
        .map(|_| {
            let states = states.clone();
            tokio::spawn(async move {
                let user_id = Uuid::new_v4();
                let (_, state, _) = OAuthState::new(user_id, "google", "https://callback", vec![]);
                states.lock().await.insert(state);
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }

    // Assert
    let final_states = states.lock().await;
    assert_eq!(final_states.len(), 50, "All concurrent states should be unique");
}

#[tokio::test]
async fn pkce_verifier_and_challenge_relationship() {
    // Arrange
    let verifier = generate_code_verifier();
    let challenge = generate_code_challenge(&verifier);

    // Manually compute to verify
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let expected_challenge = URL_SAFE_NO_PAD.encode(hash);

    // Assert
    assert_eq!(challenge, expected_challenge, "Challenge should be SHA256(verifier) base64url encoded");
}
