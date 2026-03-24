//! System node agent service — shared domain logic for configuring
//! runtime agent systems from a file-based repository.
//!
//! Used by:
//! - `SystemNodeStrategy` (execution strategy)
//! - Sync step (files → DB projection)
//! - Future workflow-level system agents

pub mod file_reader;
pub mod state;
pub mod sync;
pub mod validate;
