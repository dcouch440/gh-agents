//! Tests for authentication utilities

use super::*;

#[test]
fn test_hash_and_verify_password() {
    let password = "test_password_123";
    let hash = hash_password(password).expect("Failed to hash password");

    assert!(verify_password(password, &hash));
    assert!(!verify_password("wrong_password", &hash));
}

#[test]
fn test_create_and_verify_token() {
    let secret = b"test_secret_key_123";
    let user_id = UserId::new();
    let token = create_token(secret, 24, user_id, "test@example.com", false)
        .expect("Failed to create token");

    let claims = verify_token(&token, secret).expect("Failed to verify token");
    assert!(uuid::Uuid::parse_str(&claims.sub).is_ok());
    assert_eq!(claims.email, "test@example.com");
    assert!(claims.exp > claims.iat);
}

#[test]
fn test_verify_token_wrong_secret() {
    let secret = b"test_secret_key_123";
    let wrong_secret = b"wrong_secret_key";
    let user_id = UserId::new();
    let token = create_token(secret, 24, user_id, "test@example.com", false)
        .expect("Failed to create token");

    assert!(verify_token(&token, wrong_secret).is_err());
}

// =====================================================================
// JWT Edge Case Tests
// =====================================================================

#[test]
fn test_expired_token_rejected() {
    let secret = b"test_secret_key_123";
    let expired_claims = Claims {
        sub: Uuid::new_v4().to_string(),
        email: "test@example.com".to_string(),
        is_admin: false,
        exp: 1, // epoch + 1 second = long expired
        iat: 0,
    };
    let token = encode(
        &Header::default(),
        &expired_claims,
        &EncodingKey::from_secret(secret),
    )
    .unwrap();

    assert!(verify_token(&token, secret).is_err());
}

#[test]
fn test_malformed_jwt_corrupted() {
    let secret = b"test_secret_key_123";
    assert!(verify_token("not.valid.jwt", secret).is_err());
}

#[test]
fn test_malformed_jwt_no_dots() {
    let secret = b"test_secret_key_123";
    assert!(verify_token("nodotshere", secret).is_err());
}

#[test]
fn test_malformed_jwt_empty() {
    let secret = b"test_secret_key_123";
    assert!(verify_token("", secret).is_err());
}

#[test]
fn test_malformed_jwt_whitespace() {
    let secret = b"test_secret_key_123";
    assert!(verify_token("   ", secret).is_err());
}

#[test]
fn test_missing_sub_claim() {
    let secret = b"test_secret_key_123";

    #[derive(Serialize)]
    struct ClaimsNoSub {
        email: String,
        exp: usize,
        iat: usize,
    }

    let partial = ClaimsNoSub {
        email: "test@example.com".to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &partial,
        &EncodingKey::from_secret(secret),
    )
    .unwrap();

    assert!(verify_token(&token, secret).is_err());
}

#[test]
fn test_missing_email_claim() {
    let secret = b"test_secret_key_123";

    #[derive(Serialize)]
    struct ClaimsNoEmail {
        sub: String,
        exp: usize,
        iat: usize,
    }

    let partial = ClaimsNoEmail {
        sub: Uuid::new_v4().to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &partial,
        &EncodingKey::from_secret(secret),
    )
    .unwrap();

    assert!(verify_token(&token, secret).is_err());
}

#[test]
fn test_admin_flag_true() {
    let secret = b"test_secret_key_123";
    let user_id = UserId::new();
    let token = create_token(secret, 24, user_id, "admin@example.com", true).unwrap();

    let claims = verify_token(&token, secret).unwrap();
    assert!(claims.is_admin);
    assert_eq!(claims.email, "admin@example.com");
}

#[test]
fn test_admin_flag_default_false() {
    let secret = b"test_secret_key_123";

    // Encode a struct WITHOUT is_admin — the #[serde(default)] on Claims
    // should decode it as false.
    #[derive(Serialize)]
    struct ClaimsWithoutAdmin {
        sub: String,
        email: String,
        exp: usize,
        iat: usize,
    }

    let partial = ClaimsWithoutAdmin {
        sub: Uuid::new_v4().to_string(),
        email: "user@example.com".to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &partial,
        &EncodingKey::from_secret(secret),
    )
    .unwrap();

    let claims = verify_token(&token, secret).unwrap();
    assert!(!claims.is_admin);
}

#[test]
fn test_future_iat_accepted() {
    // jsonwebtoken's default Validation does NOT check iat,
    // so a token with iat far in the future should still verify.
    let secret = b"test_secret_key_123";
    let future_iat = Claims {
        sub: Uuid::new_v4().to_string(),
        email: "future@example.com".to_string(),
        is_admin: false,
        exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
        iat: (chrono::Utc::now() + chrono::Duration::hours(48)).timestamp() as usize,
    };
    let token = encode(
        &Header::default(),
        &future_iat,
        &EncodingKey::from_secret(secret),
    )
    .unwrap();

    let claims = verify_token(&token, secret).unwrap();
    assert_eq!(claims.email, "future@example.com");
}
