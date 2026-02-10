//! Documenter protocol strategies — phases of the document generation pipeline.
//!
//! - **Coordinator** (Phase 1): single-turn structured planning
//! - **Research** (Phase 2): multi-round tool-using research
//! - **Writer** (Phase 3): single-turn document generation

pub mod coordinator;
pub mod research;
pub mod writer;

pub use coordinator::{DocumenterCoordinatorConfig, DocumenterCoordinatorStrategy};
pub use research::{DocumenterResearchConfig, DocumenterResearchStrategy};
pub use writer::{DocumenterWriterConfig, DocumenterWriterStrategy};

mod tests;
