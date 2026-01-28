//! Task orchestration and scheduling
//!
//! This module provides:
//! - `Planner` - Decomposes tickets into vertical slices
//! - `TaskQueue` / `PersistentTaskQueue` - Priority task queue with persistence
//! - `Router` - Routes tasks to appropriate agent tiers
//! - `DependencyTracker` - Tracks task dependencies
//! - `Scheduler` - Controls work assignment based on production mode

mod dependency;
mod planner;
mod queue;
mod router;
mod scheduler;

pub use dependency::{DependencyError, DependencyTracker};
pub use planner::{DecompositionError, DecompositionResult, Planner, PlannerConfig, PlannerOutput};
pub use queue::{
    DependencyAwareQueue, PersistentTaskQueue, QueueError, QueueStats, RequeuePolicy, TaskQueue,
};
pub use router::{Router, RouterConfig, RoutingRule, RuleMatcher};
pub use scheduler::{PreemptionAction, Scheduler, SchedulerConfig, SchedulerError, TaskScheduler};
