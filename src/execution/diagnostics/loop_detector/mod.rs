//! Loop detector — tracks per-file edit history and breaks escalation cycles.
//!
//! Agents that don't solve by turn 10 almost always spiral. The loop detector
//! catches the pattern early: same file edited 3+ times → Info, 5+ → Warning.

mod tests;

use std::collections::HashMap;
use std::path::PathBuf;

use super::types::{ChangeType, FileChange};

/// Tracks per-file edit history within an agent's execution.
pub struct LoopDetector {
    /// Map from file path to list of edit records.
    file_edits: HashMap<PathBuf, Vec<EditRecord>>,
}

struct EditRecord {
    command_index: usize,
    size: u64,
}

/// Loop detection status for the current command.
#[derive(Debug)]
pub enum LoopStatus {
    /// No repeated edits detected.
    Clean,
    /// Informational: file edited 3+ times — might be fine, might be stuck.
    Info {
        file: PathBuf,
        edit_count: usize,
        message: String,
    },
    /// Warning: file edited 5+ times — very likely stuck in a loop.
    Warning {
        file: PathBuf,
        edit_count: usize,
        message: String,
    },
}

impl LoopStatus {
    /// Whether this status should be rendered in the envelope.
    pub fn should_render(&self) -> bool {
        !matches!(self, LoopStatus::Clean)
    }

    /// Render the loop status for the LLM.
    pub fn render(&self) -> String {
        match self {
            LoopStatus::Clean => String::new(),
            LoopStatus::Info { message, .. } | LoopStatus::Warning { message, .. } => {
                message.clone()
            }
        }
    }
}

impl LoopDetector {
    pub fn new() -> Self {
        Self {
            file_edits: HashMap::new(),
        }
    }

    /// Record file changes and return the loop status.
    ///
    /// Only tracks `Modified` files — `Created` and `Deleted` don't indicate loops.
    pub fn record(&mut self, command_index: usize, changes: &[FileChange]) -> LoopStatus {
        let mut max_status = LoopStatus::Clean;

        for change in changes {
            if change.change_type != ChangeType::Modified {
                continue;
            }

            let edits = self.file_edits.entry(change.path.clone()).or_default();
            edits.push(EditRecord {
                command_index,
                size: change.size,
            });

            let count = edits.len();
            if count >= 5 {
                // Always upgrade to Warning for 5+
                max_status = LoopStatus::Warning {
                    file: change.path.clone(),
                    edit_count: count,
                    message: format!(
                        "LOOP DETECTED: {} has been edited {} times this step. \
                         Consider reading the full file, then either rewrite it completely \
                         or rollback to an earlier approach.",
                        change.path.display(),
                        count
                    ),
                };
            } else if count >= 3 && !matches!(max_status, LoopStatus::Warning { .. }) {
                max_status = LoopStatus::Info {
                    file: change.path.clone(),
                    edit_count: count,
                    message: format!(
                        "{} edited {} times — consider reading the full file before the next edit.",
                        change.path.display(),
                        count
                    ),
                };
            }
        }

        max_status
    }
}
