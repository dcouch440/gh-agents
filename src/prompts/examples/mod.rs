//! Few-shot examples library for teaching agents through demonstration.
//!
//! This module provides curated examples for:
//! - Decomposition: Vertical slicing patterns for different domains
//! - Implementation: Plan-then-code patterns with context requests
//! - Review: Constructive feedback with different verdicts
//! - Recovery: Graceful failure handling and escalation

mod decomposition;
mod implementation;
mod recovery;
mod review;
mod selector;

pub use decomposition::*;
pub use implementation::*;
pub use recovery::*;
pub use review::*;
pub use selector::*;
