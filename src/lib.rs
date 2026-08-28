//! nexor: AI Agent Orchestration for GitHub Workflows

// The `feature/{mod.rs, tests.rs}` convention (see CLAUDE.md) puts a
// `mod tests { .. }` block inside a file already declared as `mod tests`,
// which clippy flags as module inception. The layout is deliberate.
#![allow(clippy::module_inception)]

pub mod cli;
pub mod commands;
pub mod config;
pub mod constants;
pub mod db;
pub mod env;
pub mod error;
pub mod execution;
pub mod github;
pub mod llm;
pub mod logging;
pub mod markup;
pub mod net;
pub mod prompts;
pub mod server;
pub mod tools;
pub mod types;
