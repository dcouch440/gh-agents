//! Pipeline service: semantic CRUD for child workflow (pipeline) management.
//!
//! A **pipeline** is a child workflow attached to a parent step via
//! `child_workflow_id`. It contains an ordered set of steps connected
//! by dependency edges, with an optional Designer pre-phase step.
//!
//! This service provides the building blocks for:
//! - **Workforce tools** — interactive pipeline configuration via chat
//! - **Protocol apply** — programmatic pipeline creation from protocol expansion
//! - **Future protocols** — any system that needs to compose multi-step pipelines

pub mod types;

mod add_edge;
mod add_step;
mod create;
mod cycle;
mod destroy;
mod recompute;
mod remove_edge;
mod remove_step;
mod snapshot;
mod update_step;

mod tests;

pub use add_edge::add_edge;
pub use add_step::add_step;
pub use create::create_pipeline;
pub use cycle::would_create_cycle;
pub use destroy::destroy_pipeline;
pub use recompute::recompute_execution_order;
pub use remove_edge::remove_edge;
pub use remove_step::remove_step;
pub use snapshot::build_snapshot;
pub use types::*;
pub use update_step::update_step;
