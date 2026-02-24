//! Service layer: domain logic shared between API handlers and internal callers.
//!
//! Each sub-module exposes free functions that take repo trait references and
//! explicit parameters (no HTTP types). Handlers call these functions after
//! parsing HTTP requests; the background planner calls them directly.

pub mod agent_context;
pub mod agent_executions;
pub mod agent_roster;
pub mod agents;
pub mod board;
pub mod chat;
pub mod collections;
pub mod costs;
pub mod dispatch;
pub mod documents;
pub mod edges;
pub mod error;
pub mod messaging;
pub mod output_schemas;
pub mod ownership;
pub mod pipeline;
pub mod prompt_templates;
pub mod protocols;
pub mod results;
pub mod rooms;
pub mod routing_rules;
pub mod sessions;
pub mod step_ports;
pub mod steps;
pub mod system_config;
pub mod timeline;
pub mod tools;
pub mod validation;
pub mod workflows;

pub use error::ServiceError;
