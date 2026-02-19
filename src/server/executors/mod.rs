//! Executor modules for different execution contexts.
//!
//! - `chat`: Background worker for chat messages
//! - `collection_dag`: Multi-workflow orchestration

pub mod chat;
pub mod collection_dag;
pub mod dispatch;

pub use chat::*;
pub use collection_dag::*;
