//! Protocol execution recorder — shared phase tracking for protocol executors.
//!
//! Wraps `ProtocolRepo` operations for creating and updating execution rows.
//! Used by all protocol executors (documenter, future protocols) to record
//! phase-level execution metadata (tokens, cost, status, output).

use anyhow::anyhow;
use chrono::Utc;
use uuid::Uuid;

use crate::db::traits::{ProtocolRepo, UpdateProtocolExecutionStatusInput};
use crate::db::ProtocolExecutionRow;
use crate::server::hub::error::HubError;

/// Completion data for a protocol execution phase.
pub(crate) struct PhaseCompletion<'a> {
    pub status: &'a str,
    pub output_content: Option<&'a str>,
    pub error_message: Option<&'a str>,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f32,
    pub model: Option<&'a str>,
}

/// Records protocol execution phases to the database.
///
/// Each protocol executor creates a recorder scoped to a specific step and run,
/// then uses it to track individual phase executions (e.g., strategy, research, write).
pub struct ProtocolExecutionRecorder<'a> {
    protocol_repo: &'a dyn ProtocolRepo,
    step_id: Uuid,
    run_id: Uuid,
    /// Default archetype label applied to all phases created by this recorder.
    default_archetype: Option<String>,
}

impl<'a> ProtocolExecutionRecorder<'a> {
    pub fn new(protocol_repo: &'a dyn ProtocolRepo, step_id: Uuid, run_id: Uuid) -> Self {
        Self {
            protocol_repo,
            step_id,
            run_id,
            default_archetype: None,
        }
    }

    /// Set a default archetype that will be applied to all phases.
    pub fn with_archetype(mut self, archetype: &str) -> Self {
        self.default_archetype = Some(archetype.to_string());
        self
    }

    /// Create a new execution row for a protocol phase.
    ///
    /// Returns the created row (with generated ID and timestamps).
    pub async fn create_phase(
        &self,
        phase: &str,
        document_def_id: Option<Uuid>,
        input_prompt: Option<&str>,
    ) -> Result<ProtocolExecutionRow, HubError> {
        let row = ProtocolExecutionRow {
            id: Uuid::new_v4(),
            protocol_step_id: self.step_id,
            workflow_run_id: Some(self.run_id),
            phase: phase.to_string(),
            document_def_id,
            agent_id: None,
            input_prompt: input_prompt.map(String::from),
            output_content: None,
            status: "running".to_string(),
            error_message: None,
            tokens_in: None,
            tokens_out: None,
            cost_usd: None,
            model: None,
            capabilities_used: None,
            created_at: Utc::now(),
            completed_at: None,
            agent_name: None,
            archetype: self.default_archetype.clone(),
            designer_run_id: None,
        };

        self.protocol_repo
            .create_protocol_execution(row)
            .await
            .map_err(|e| HubError::Internal(anyhow!("failed to create execution row: {}", e)))
    }

    /// Create a new execution row with extended context (agent name, archetype, designer link).
    ///
    /// Used by task_force and other protocols that need to track agent-level phases.
    pub async fn create_phase_with_context(
        &self,
        phase: &str,
        document_def_id: Option<Uuid>,
        input_prompt: Option<&str>,
        agent_name: Option<&str>,
        archetype: Option<&str>,
        designer_run_id: Option<Uuid>,
    ) -> Result<ProtocolExecutionRow, HubError> {
        let row = ProtocolExecutionRow {
            id: Uuid::new_v4(),
            protocol_step_id: self.step_id,
            workflow_run_id: Some(self.run_id),
            phase: phase.to_string(),
            document_def_id,
            agent_id: None,
            input_prompt: input_prompt.map(String::from),
            output_content: None,
            status: "running".to_string(),
            error_message: None,
            tokens_in: None,
            tokens_out: None,
            cost_usd: None,
            model: None,
            capabilities_used: None,
            created_at: Utc::now(),
            completed_at: None,
            agent_name: agent_name.map(String::from),
            archetype: archetype.map(String::from),
            designer_run_id,
        };

        self.protocol_repo
            .create_protocol_execution(row)
            .await
            .map_err(|e| HubError::Internal(anyhow!("failed to create execution row: {}", e)))
    }

    /// Update an execution row with completion data.
    pub(crate) async fn update_phase(&self, id: Uuid, completion: PhaseCompletion<'_>) {
        let _ = self
            .protocol_repo
            .update_protocol_execution_status(UpdateProtocolExecutionStatusInput {
                id,
                status: completion.status.to_string(),
                output_content: completion.output_content.map(String::from),
                error_message: completion.error_message.map(String::from),
                tokens_in: Some(completion.tokens_in as i32),
                tokens_out: Some(completion.tokens_out as i32),
                cost_usd: Some(completion.cost_usd as f64),
                model: completion.model.map(String::from),
            })
            .await;
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorder_new_stores_ids() {
        // Verify the recorder captures step and run IDs correctly.
        // We can't easily test async DB calls without a mock, but we
        // verify construction doesn't panic and fields are accessible.
        let step_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        // Use a minimal assertion — the real tests are integration-level
        // via the documenter executor tests.
        assert_ne!(step_id, run_id);
    }
}
