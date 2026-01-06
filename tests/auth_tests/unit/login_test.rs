///! Unit Tests for User Login and Password Handling

use echo_backend::modules::auth::services::{
    hash_password, validate_password_strength, verify_password,
};

// =============================================================================
// PASSWORD HASHING TESTS
// =============================================================================

#[tokio::test]
async fn hash_password_produces_argon2_hash() {
    let password = "SecurePass123!";

    let hash = hash_password(password).unwrap();

    assert!(hash.starts_with("$argon2"), "Hash should be Argon2 format");
}

#[tokio::test]
async fn hash_password_produces_unique_hashes() {
    let password = "SecurePass123!";

    let hash1 = hash_password(password).unwrap();
    let hash2 = hash_password(password).unwrap();

    assert_ne!(hash1, hash2, "Same password should produce different hashes (unique salt)");
}

#[tokio::test]
async fn verify_password_accepts_correct_password() {
    let password = "SecurePass123!";
    let hash = hash_password(password).unwrap();

    let result = verify_password(password, &hash).unwrap();

    assert!(result, "Correct password should verify successfully");
}

#[tokio::test]
async fn verify_password_rejects_wrong_password() {
    let password = "SecurePass123!";
    let wrong_password = "WrongPassword456!";
    let hash = hash_password(password).unwrap();

    let result = verify_password(wrong_password, &hash).unwrap();

    assert!(!result, "Wrong password should fail verification");
}

#[tokio::test]
async fn verify_password_is_case_sensitive() {
    let password = "SecurePass123!";
    let wrong_case = "securepass123!";
    let hash = hash_password(password).unwrap();

    let result = verify_password(wrong_case, &hash).unwrap();

    assert!(!result, "Password verification should be case sensitive");
}

#[tokio::test]
async fn verify_password_rejects_empty_password() {
    let password = "SecurePass123!";
    let hash = hash_password(password).unwrap();

    let result = verify_password("", &hash).unwrap();

    assert!(!result, "Empty password should fail verification");
}

#[tokio::test]
async fn verify_password_handles_unicode() {
    let password = "Sécure密码123!";
    let hash = hash_password(password).unwrap();

    let result = verify_password(password, &hash).unwrap();

    assert!(result, "Unicode password should verify correctly");
}

#[tokio::test]
async fn verify_password_rejects_invalid_hash() {
    let result = verify_password("anypassword", "invalid_hash_format");

    assert!(result.is_err(), "Invalid hash should return error");
}

// =============================================================================
// PASSWORD STRENGTH VALIDATION TESTS
// =============================================================================

#[tokio::test]
async fn validate_password_strength_accepts_strong_password() {
    let result = validate_password_strength("SecurePass123!");

    assert!(result.is_ok(), "Strong password should be accepted");
}

#[tokio::test]
async fn validate_password_strength_rejects_short_password() {
    let result = validate_password_strength("Short1!");

    assert!(result.is_err(), "Short password should be rejected");
}

#[tokio::test]
async fn validate_password_strength_accepts_8_char_minimum() {
    let result = validate_password_strength("Abcd1234");

    assert!(result.is_ok(), "8 character password should be accepted");
}

#[tokio::test]
async fn validate_password_strength_rejects_7_char_password() {
    let result = validate_password_strength("Abcd123");

    assert!(result.is_err(), "7 character password should be rejected");
}

#[tokio::test]
async fn validate_password_strength_rejects_too_long_password() {
    let long_password = "A1".to_string() + &"a".repeat(127);

    let result = validate_password_strength(&long_password);

    assert!(result.is_err(), "Password over 128 chars should be rejected");
}

#[tokio::test]
async fn validate_password_strength_accepts_128_char_password() {
    let password = "A1".to_string() + &"a".repeat(126);
    assert_eq!(password.len(), 128);

    let result = validate_password_strength(&password);

    assert!(result.is_ok(), "128 character password should be accepted");
}

#[tokio::test]
async fn validate_password_strength_rejects_no_letter() {
    let result = validate_password_strength("12345678!");

    assert!(result.is_err(), "Password without letter should be rejected");
}

#[tokio::test]
async fn validate_password_strength_rejects_no_number() {
    let result = validate_password_strength("SecurePass!");

    assert!(result.is_err(), "Password without number should be rejected");
}

#[tokio::test]
async fn validate_password_strength_rejects_empty_password() {
    let result = validate_password_strength("");

    assert!(result.is_err(), "Empty password should be rejected");
}

// =============================================================================
// PASSWORD TIMING TESTS
// =============================================================================

#[tokio::test]
async fn password_verification_timing_is_consistent() {
    let password = "SecurePass123!";
    let hash = hash_password(password).unwrap();

    // Measure time for correct password
    let start = std::time::Instant::now();
    let _ = verify_password(password, &hash);
    let correct_time = start.elapsed();

    // Measure time for wrong password
    let start = std::time::Instant::now();
    let _ = verify_password("WrongPassword!", &hash);
    let wrong_time = start.elapsed();

    // Times should be roughly similar (within 50ms for Argon2)
    let diff = if correct_time > wrong_time {
        correct_time - wrong_time
    } else {
        wrong_time - correct_time
    };

    assert!(
        diff < std::time::Duration::from_millis(100),
        "Timing difference should be minimal to prevent timing attacks"
    );
}

// =============================================================================
// HASH FORMAT TESTS
// =============================================================================

#[tokio::test]
async fn hash_contains_salt() {
    let password = "SecurePass123!";
    let hash = hash_password(password).unwrap();

    // Argon2 hash format includes salt
    let parts: Vec<&str> = hash.split('$').collect();
    assert!(parts.len() >= 5, "Argon2 hash should have multiple parts including salt");
}

#[tokio::test]
async fn hash_is_not_plaintext() {
    let password = "SecurePass123!";
    let hash = hash_password(password).unwrap();

    assert!(!hash.contains(password), "Hash should not contain plaintext password");
}

#[tokio::test]
async fn hash_is_deterministic_per_salt() {
    let password = "SecurePass123!";
    let hash = hash_password(password).unwrap();

    // Same hash should verify the same password
    assert!(verify_password(password, &hash).unwrap());
    assert!(verify_password(password, &hash).unwrap());
    assert!(verify_password(password, &hash).unwrap());
}
