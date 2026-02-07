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
    #[serde(default)]
    pub is_admin: bool,
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
    is_admin: bool,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now();
    let exp = (now + chrono::Duration::hours(duration_hours as i64)).timestamp() as usize;

    let claims = Claims {
        sub: user_id.0.to_string(),
        email: email.to_string(),
        is_admin,
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
                parts.uri.query().and_then(|q| {
                    q.split('&')
                        .find_map(|pair| pair.strip_prefix("token=").map(|v| v.to_string()))
                })
            })
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let token = &token;

        let claims =
            verify_token(token, &state.jwt_secret()).map_err(|_| StatusCode::UNAUTHORIZED)?;

        let user_id = claims
            .sub
            .parse::<Uuid>()
            .map(UserId)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser { user_id, claims })
    }
}

#[cfg(test)]
mod tests;
