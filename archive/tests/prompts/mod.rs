//! Prompt testing infrastructure for nexor.
//!
//! This module provides testing tools for prompts:
//! - `harness` - Test harness for running prompts
//! - `assertions` - Custom assertions for prompt output
//! - `diff` - Diff tooling for comparing outputs
//! - `confusion` - Confusion detection for LLM output

pub mod assertions;
pub mod confusion;
pub mod diff;
pub mod harness;
