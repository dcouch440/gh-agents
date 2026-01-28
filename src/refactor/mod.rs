//! Refactor mode for mid-stream plan modifications.
//!
//! This module provides:
//! - `RefactorAgent` - Agent for handling refactor conversations
//! - Intent detection from user messages
//! - Change proposal generation
//! - Change application logic

mod agent;

pub use agent::*;
