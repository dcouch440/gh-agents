//! Domain-level error type for service functions.
//!
//! Maps cleanly to `AppError` but carries no HTTP semantics,
//! allowing services to be called from both API handlers and
//! internal callers (e.g. background planner agent).

/// Domain-level error returned by service functions.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    /// Input validation failure.
    #[error("{0}")]
    Validation(String),

    /// Resource not found (or ownership mismatch — treated identically to
    /// avoid confirming resource existence to unauthorized callers).
    #[error("{0} not found")]
    NotFound(String),

    /// Caller lacks permission for the requested operation.
    #[error("{0}")]
    Forbidden(String),

    /// Resource state conflict (e.g. duplicate, invalid transition).
    #[error("{0}")]
    Conflict(String),

    /// Unexpected internal failure.
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ServiceError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn not_found(resource: &str) -> Self {
        Self::NotFound(resource.to_string())
    }
}
