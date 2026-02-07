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
