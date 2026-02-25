//! Execution infrastructure — engine, strategies, recording, streaming.
//!
//! All LLM execution flows through `ExecutionEngine::execute(&strategy)`.
//! Strategies parameterize the engine with system prompts, tools, and handlers.

pub mod engine;
pub mod recorder;
pub mod strategies;
pub mod strategy;
pub mod streaming;
