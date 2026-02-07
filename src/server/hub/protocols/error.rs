//! Protocol Layer error types.

/// Errors that can occur during protocol operations.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Unknown protocol type: {0}")]
    UnknownType(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Protocol requires at least one port")]
    NoPorts,

    #[error("Duplicate port name: {0}")]
    DuplicatePortName(String),

    #[error("Invalid port name \"{0}\": must match [a-z][a-z0-9_]* and be at most 50 characters")]
    InvalidPortName(String),

    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),
}
