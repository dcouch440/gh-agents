//! Shared authorization helpers for API handlers.
//!
//! Ownership verification is centralized in `services::ownership`.

use super::AppError;
use crate::server::auth::AuthUser;

/// Verify the authenticated user is an admin.
///
/// Returns 404 on non-admin (avoids confirming endpoint existence to
/// regular users).
pub fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if !auth.claims.is_admin {
        return Err(AppError::not_found("Resource"));
    }
    Ok(())
}
