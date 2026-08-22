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
//! Unauthenticated requests fall back to the IP so an anonymous flood is still
//! bounded. Those requests are rejected by the auth layer immediately after, so
//! the fallback only has to survive the gap between the two.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;

use axum::http::{header::AUTHORIZATION, Request};
use tower_governor::errors::GovernorError;
use tower_governor::key_extractor::{KeyExtractor, SmartIpKeyExtractor};

mod tests;

/// The bucket a request is counted against.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitKey {
    /// Hash of the caller's bearer token — never the token itself, so the
    /// limiter's key map holds no credentials even in a memory dump.
    Session(u64),
    /// No usable token on the request.
    Ip(IpAddr),
}

/// Keys by bearer token when there is one, else by client IP.
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionOrIpKeyExtractor;

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
/// and does not outlive a restart. Tokens are JWTs we signed, so a caller cannot
/// choose their own bytes to force a collision into someone else's bucket.
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
        match extract_token(req) {
            Some(token) => Ok(RateLimitKey::Session(hash_token(token))),
            None => SmartIpKeyExtractor.extract(req).map(RateLimitKey::Ip),
        }
    }
}
