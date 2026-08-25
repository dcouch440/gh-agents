//! Rate-limit key extraction.
//!
//! Protected routes are keyed by the caller's session rather than their IP.
//! `SmartIpKeyExtractor` falls back to the peer address whenever no
//! `X-Forwarded-For` is present, which is exactly what happens behind a Docker
//! bridge network or a proxy that has not been configured to forward it — every
//! user collapses onto the gateway's address and shares one bucket. One active
//! user can then exhaust the quota for everyone, and the victim sees an
//! unexplained failure rather than anything attributable to their own usage.
//!
//! Keying by session makes the configured limits mean what they say: `20/s` is
//! twenty requests per second *per signed-in session*, not twenty for the whole
//! deployment.
//!
//! A session bucket is only granted to a token whose signature verifies. That
//! check is what makes the whole scheme safe: this layer runs *before* the auth
//! extractor, so if any bearer string were taken at face value a caller could
//! send a fresh random token per request, land in a fresh bucket every time and
//! never be limited at all. Anything that does not verify — junk, expired,
//! forged — falls back to the IP bucket, which is exactly where unauthenticated
//! traffic belongs.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;
use std::sync::Arc;

use axum::http::{header::AUTHORIZATION, Request};
use tower_governor::errors::GovernorError;
use tower_governor::key_extractor::{KeyExtractor, SmartIpKeyExtractor};

use crate::server::auth::verify_token;

mod tests;

/// The bucket a request is counted against.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitKey {
    /// Hash of a *verified* bearer token — never the token itself, so the
    /// limiter's key map holds no credentials even in a memory dump.
    Session(u64),
    /// No token, or one that did not verify.
    Ip(IpAddr),
}

/// Keys by verified bearer token when there is one, else by client IP.
///
/// Holds the JWT secret because the key has to be decided before the auth
/// extractor has run — there is no `AuthUser` in the request at this point, and
/// trusting an unverified token would hand every caller an unlimited supply of
/// buckets.
#[derive(Clone)]
pub struct SessionOrIpKeyExtractor {
    jwt_secret: Arc<[u8]>,
}

impl SessionOrIpKeyExtractor {
    pub fn new(jwt_secret: &[u8]) -> Self {
        Self {
            jwt_secret: Arc::from(jwt_secret),
        }
    }
}

impl std::fmt::Debug for SessionOrIpKeyExtractor {
    /// Hand-written so the JWT secret cannot reach a log line through a derived
    /// `Debug` on this or on anything holding it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionOrIpKeyExtractor")
    }
}

/// Pull the bearer token exactly the way [`crate::server::auth::AuthUser`] does:
/// the `Authorization` header first, then the `?token=` query parameter that
/// `EventSource` has to use because it cannot set headers.
///
/// Kept deliberately in step with the extractor — if the two disagree about what
/// counts as a token, a caller could be limited under one key and authenticated
/// as another.
fn extract_token<T>(req: &Request<T>) -> Option<&str> {
    let from_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .filter(|s| !s.is_empty());

    if from_header.is_some() {
        return from_header;
    }

    req.uri()
        .query()
        .and_then(|q| q.split('&').find_map(|pair| pair.strip_prefix("token=")))
        .filter(|s| !s.is_empty())
}

/// Hash a token down to a fixed-size key.
///
/// Only has to be stable within one process — the limiter's state is in-memory
/// and does not outlive a restart. `DefaultHasher` is deliberately seeded with
/// fixed keys, so it is not collision-resistant against chosen input; that is
/// acceptable only because callers reach this function exclusively with tokens
/// we signed. Grinding a collision to land in someone else's bucket would first
/// require forging a JWT.
fn hash_token(token: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    hasher.finish()
}

impl KeyExtractor for SessionOrIpKeyExtractor {
    type Key = RateLimitKey;

    // `name`/`key_name` are gated behind tower_governor's `tracing` feature,
    // which we do not enable. Turning it on will surface here as a missing-method
    // error rather than anything subtle.
    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        // An HS256 verify is a single HMAC over a short string — cheap enough to
        // run per request, and the auth extractor repeats it moments later
        // anyway.
        match extract_token(req).filter(|t| verify_token(t, &self.jwt_secret).is_ok()) {
            Some(token) => Ok(RateLimitKey::Session(hash_token(token))),
            None => SmartIpKeyExtractor.extract(req).map(RateLimitKey::Ip),
        }
    }
}
