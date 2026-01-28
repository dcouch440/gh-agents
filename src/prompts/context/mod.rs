//! Context management for prompts.
//!
//! This module provides:
//! - `ContextBudget` - Token budget allocation per category
//! - `ContextInjector` - Priority-based context injection
//! - `ContextCategory` - Categories for budget allocation
//! - `FileSelector` - Smart file selection by relevance
//! - `FileSummarizer` - Large file summarization

mod injector;
mod manager;
mod summarizer;

pub use injector::*;
pub use manager::*;
pub use summarizer::*;
