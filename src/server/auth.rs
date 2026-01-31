//! Authentication handling

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::state::AppState;
use crate::types::UserId;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn create_token(
    secret: &[u8],
    duration_hours: u64,
    user_id: UserId,
    email: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now();
    let exp = (now + chrono::Duration::hours(duration_hours as i64)).timestamp() as usize;

    let claims = Claims {
        sub: user_id.0.to_string(),
        email: email.to_string(),
        exp,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
}

pub fn verify_token(token: &str, secret: &[u8]) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Authenticated user extractor for protected routes
///
/// Extracts and validates JWT token from Authorization header.
/// Returns 401 Unauthorized if token is missing or invalid.
pub struct AuthUser {
    pub user_id: UserId,
    pub claims: Claims,
}

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Try Authorization header first, then fall back to ?token= query param
        // (needed for EventSource/SSE which cannot set custom headers)
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.to_string())
            .or_else(|| {
                parts
                    .uri
                    .query()
                    .and_then(|q| {
                        q.split('&')
                            .find_map(|pair| {
                                pair.strip_prefix("token=").map(|v| v.to_string())
                            })
                    })
            })
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let token = &token;

        let claims =
            verify_token(token, &state.jwt_secret).map_err(|_| StatusCode::UNAUTHORIZED)?;

        let user_id = claims
            .sub
            .parse::<Uuid>()
            .map(UserId)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser { user_id, claims })
    }
}

#[cfg(test)]
mod tests {
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
        let token = create_token(secret, 24, user_id, "test@example.com")
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
        let token = create_token(secret, 24, user_id, "test@example.com")
            .expect("Failed to create token");

        assert!(verify_token(&token, wrong_secret).is_err());
    }
}
