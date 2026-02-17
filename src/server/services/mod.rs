//! Service layer: domain logic shared between API handlers and internal callers.
//!
//! Each sub-module exposes free functions that take repo trait references and
//! explicit parameters (no HTTP types). Handlers call these functions after
//! parsing HTTP requests; the background planner calls them directly.

pub mod agents;
pub mod edges;
pub mod error;
pub mod steps;
pub mod validation;
pub mod workflows;

pub use error::ServiceError;
