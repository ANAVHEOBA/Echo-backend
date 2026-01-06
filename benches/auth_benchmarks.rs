use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

// Benchmark password hashing (this is CPU-intensive)
fn benchmark_password_hashing(c: &mut Criterion) {
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2, PasswordHasher,
    };

    c.bench_function("argon2_hash_password", |b| {
        let argon2 = Argon2::default();
        let password = b"test_password_123";

        b.iter(|| {
            let salt = SaltString::generate(&mut OsRng);
            let hash = argon2
                .hash_password(black_box(password), &salt)
                .expect("Failed to hash password");
            hash.to_string() // Return owned String instead of reference
        })
    });
}

// Benchmark password verification
fn benchmark_password_verification(c: &mut Criterion) {
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    };

    let argon2 = Argon2::default();
    let password = b"test_password_123";
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2
        .hash_password(password, &salt)
        .expect("Failed to hash")
        .to_string();

    c.bench_function("argon2_verify_password", |b| {
        let parsed_hash = PasswordHash::new(&hash).expect("Failed to parse hash");

        b.iter(|| {
            argon2
                .verify_password(black_box(password), &parsed_hash)
                .expect("Failed to verify")
        })
    });
}

// Benchmark JWT token creation
fn benchmark_jwt_creation(c: &mut Criterion) {
    use chrono::Utc;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: Uuid,
        email: String,
        role: String,
        exp: i64,
        iat: i64,
    }

    let secret = "test-secret-key-for-benchmarking-purposes";
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    c.bench_function("jwt_create_token", |b| {
        b.iter(|| {
            let claims = Claims {
                sub: Uuid::new_v4(),
                email: "test@example.com".to_string(),
                role: "member".to_string(),
                exp: Utc::now().timestamp() + 900,
                iat: Utc::now().timestamp(),
            };

            encode(&Header::default(), black_box(&claims), &encoding_key)
                .expect("Failed to create token")
        })
    });
}

// Benchmark JWT token validation
fn benchmark_jwt_validation(c: &mut Criterion) {
    use chrono::Utc;
    use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: Uuid,
        email: String,
        role: String,
        exp: i64,
        iat: i64,
    }

    let secret = "test-secret-key-for-benchmarking-purposes";
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());

    let claims = Claims {
        sub: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        role: "member".to_string(),
        exp: Utc::now().timestamp() + 3600,
        iat: Utc::now().timestamp(),
    };

    let token = encode(&Header::default(), &claims, &encoding_key).expect("Failed to create token");

    c.bench_function("jwt_validate_token", |b| {
        b.iter(|| {
            decode::<Claims>(black_box(&token), &decoding_key, &Validation::default())
                .expect("Failed to validate token")
        })
    });
}

criterion_group!(
    benches,
    benchmark_password_hashing,
    benchmark_password_verification,
    benchmark_jwt_creation,
    benchmark_jwt_validation,
);

criterion_main!(benches);
