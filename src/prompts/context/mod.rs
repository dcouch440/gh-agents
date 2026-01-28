//! Context management for prompts.
//!
//! This module provides:
//! - `ContextBudget` - Token budget allocation per category
//! - `ContextInjector` - Priority-based context injection
//! - `ContextCategory` - Categories for budget allocation
//! - `FileSelector` - Smart file selection by relevance
//! - `FileSummarizer` - Large file summarization
//! - `TokenCounter` - Token counting for context validation
//! - `ModelLimits` - Context limits for different models
//! - `ContextValidator` - Pre-flight validation before LLM calls
//! - `ContextTruncator` - Automatic truncation to fit limits
//! - `ContextPressureWarning` - Warning system for context pressure

mod injector;
mod manager;
mod summarizer;
mod validator;

pub use injector::*;
pub use manager::*;
pub use summarizer::*;
pub use validator::*;
