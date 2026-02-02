//! Unified Chat Hub — single execution engine for chat, DAG pipelines,
//! and pipeline-inside-chat flows.
//!
//! All LLM execution in the application goes through `ExecutionEngine::execute()`
//! parameterized by an `ExecutionStrategy`. Different strategies handle chat
//! sessions, DAG workflow steps, and tool routing.

pub mod engine;
pub mod error;
pub mod prompt_registry;
pub mod recorder;
pub mod streaming;
pub mod strategy;

pub use engine::{ExecutionEngine, ExecutionResult};
pub use error::HubError;
pub use prompt_registry::PromptRegistry;
pub use recorder::ExecutionRecorder;
pub use streaming::{NullSink, StreamSink};
pub use strategy::ExecutionStrategy;
