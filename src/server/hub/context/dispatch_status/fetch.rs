//! Fetch dispatch snapshots from the TaskRegistry.
//!
//! Converts in-memory `TaskEntry` records into rendering-ready
//! `DispatchSnapshot` values with truncated text and relative timestamps.

use chrono::Utc;
use uuid::Uuid;

use crate::server::hub::truncate_str;
use crate::server::state::task_registry::{TaskRegistry, TaskStatus};

use super::types::{DispatchSnapshot, DispatchStatus};

/// Fetch dispatch snapshots for a step.
///
/// Returns running tasks first, then up to 3 most recent terminal tasks.
/// Empty vec when no tasks exist for this step.
pub fn fetch(registry: &TaskRegistry, step_id: Uuid) -> Vec<DispatchSnapshot> {
    let tasks = registry.list_tasks_for_step(step_id);
    if tasks.is_empty() {
        return Vec::new();
    }

    let now = Utc::now();
    let mut snapshots = Vec::new();

    // Running tasks first
    for task in &tasks {
        if task.status != TaskStatus::Running {
            continue;
        }
        snapshots.push(DispatchSnapshot {
            id: task.execution_id.to_string()[..8].to_string(),
            instruction: truncate_str(&task.instruction, 120).to_string(),
            status: DispatchStatus::InProgress,
            elapsed: format_elapsed(now, task.created_at),
            result: None,
        });
    }

    // Up to 3 recent terminal tasks
    let recent = tasks
        .iter()
        .filter(|t| t.status != TaskStatus::Running)
        .take(3);

    for task in recent {
        let status = match task.status {
            TaskStatus::Completed => DispatchStatus::Completed,
            TaskStatus::Failed => DispatchStatus::Failed,
            TaskStatus::Cancelled => DispatchStatus::Cancelled,
            TaskStatus::Running => unreachable!(),
        };
        snapshots.push(DispatchSnapshot {
            id: task.execution_id.to_string()[..8].to_string(),
            instruction: truncate_str(&task.instruction, 120).to_string(),
            status,
            elapsed: format_elapsed(now, task.created_at),
            result: task
                .result
                .as_deref()
                .map(|r| truncate_str(r, 80).to_string()),
        });
    }

    snapshots
}

/// Format the elapsed time since `created_at` as a human-readable string.
fn format_elapsed(now: chrono::DateTime<Utc>, created_at: chrono::DateTime<Utc>) -> String {
    let secs = (now - created_at).num_seconds().max(0);

    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}
