//! Documenter executor — phased pipeline for document generation.
//!
//! Strategy → Research → Write pipeline. When the DAG encounters a step with
//! `execution_mode = "documenter"`, it dispatches to `DocumenterExecutor`
//! which runs three phases:
//!
//! 1. **Strategy** — single-turn LLM call producing a `StrategyOutput` JSON
//! 2. **Research** — parallel per-document LLM calls with capability-resolved tools
//! 3. **Write** — parallel per-document single-turn LLM calls producing final content

use std::collections::HashMap;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::WorkflowStepRow;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::execution_recorder::ProtocolExecutionRecorder;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::StepExecutionEnvelope;

use super::utils::{StepOutput, WorkflowExecutionContext};
use super::{broadcast_workflow_event, resolve_output_key};

mod persistence;
mod phases;
mod prompts;

#[cfg(test)]
mod tests;
pub(crate) mod types;

// Re-exports for external consumers and tests
pub(crate) use persistence::{determine_persist_action, DocumentPersistAction};
pub use prompts::build_documents_output;

pub(crate) use crate::server::hub::protocols::context::{build_context_block, ContextDocument};

use crate::config::protocols::DOCUMENTER;

/// Result from a complete documenter pipeline execution.
pub struct DocumenterResult {
    pub output: StepOutput,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
}

/// Phased pipeline executor for the documenter protocol.
///
/// Runs three phases sequentially (strategy), then parallel per document
/// (research, write). Records each phase as a `protocol_execution` row
/// and broadcasts WebSocket progress events.
pub(crate) struct DocumenterExecutor<'a> {
    engine: &'a ExecutionEngine,
    state: &'a AppState,
    ctx: &'a WorkflowExecutionContext,
    step: &'a WorkflowStepRow,
    prompt: &'a str,
    cancel: Option<&'a CancellationToken>,
    upstream_context: &'a [ContextDocument],
    completed_envelopes: &'a HashMap<Uuid, StepExecutionEnvelope>,
    recorder: ProtocolExecutionRecorder<'a>,
}

impl<'a> DocumenterExecutor<'a> {
    pub fn new(
        engine: &'a ExecutionEngine,
        state: &'a AppState,
        ctx: &'a WorkflowExecutionContext,
        step: &'a WorkflowStepRow,
        prompt: &'a str,
        cancel: Option<&'a CancellationToken>,
        upstream_context: &'a [ContextDocument],
        completed_envelopes: &'a HashMap<Uuid, StepExecutionEnvelope>,
    ) -> Self {
        let recorder =
            ProtocolExecutionRecorder::new(&*state.repos().protocols, step.id, ctx.run_id);
        Self {
            engine,
            state,
            ctx,
            step,
            prompt,
            cancel,
            upstream_context,
            completed_envelopes,
            recorder,
        }
    }

    /// Execute the full 3-phase documenter pipeline.
    pub async fn execute(
        &self,
        port_outputs: &std::collections::HashMap<Uuid, Vec<crate::db::StepOutputRow>>,
    ) -> Result<DocumenterResult, HubError> {
        let mut total_in: i64 = 0;
        let mut total_out: i64 = 0;
        let mut total_cost: f32 = 0.0;

        // Load document definitions for this step
        let doc_defs = self
            .state
            .repos()
            .workflows
            .list_document_defs(self.step.id)
            .await
            .map_err(|e| HubError::Internal(anyhow!("failed to load doc defs: {}", e)))?;

        if doc_defs.is_empty() {
            return Err(HubError::Internal(anyhow!(
                "documenter step {} has no document definitions",
                self.step.id
            )));
        }

        // ── Agent Designer: Strategist ──────────────────────────────────
        let strategist_designed = self.run_designer_for_strategist(&doc_defs).await;
        let (strategy_system, strategy_task) = match strategist_designed {
            Some(result) if !result.prompts.is_empty() => {
                total_in += result.input_tokens;
                total_out += result.output_tokens;
                total_cost += result.cost_usd;
                (
                    result.prompts[0].system_prompt.clone(),
                    result.prompts[0].task_prompt.clone(),
                )
            }
            _ => {
                let sys = self.load_strategy_system_prompt().await?;
                (sys, self.prompt.to_string())
            }
        };

        // Determine model from protocol config
        let model_id = DOCUMENTER.agent("strategist").model_id.clone();

        // ── Phase 1: Strategy ────────────────────────────────────────────
        let strategy_output = self
            .execute_strategy_phase(&strategy_system, &strategy_task, &model_id, &doc_defs)
            .await?;

        total_in += strategy_output.input_tokens;
        total_out += strategy_output.output_tokens;
        total_cost += strategy_output.cost_usd;

        if self.is_cancelled() {
            return Err(HubError::Internal(anyhow!(
                "documenter execution cancelled"
            )));
        }

        // Load context documents for selective injection into research/write phases
        let context_docs = self.load_context_documents().await;

        // ── Agent Designer: Researchers + Writers ────────────────────────
        let designed_lookup: HashMap<String, super::agent_designer::DesignedAgentPrompt> =
            match self
                .run_designer_for_research_write(&strategy_output.plans)
                .await
            {
                Some(result) => {
                    total_in += result.input_tokens;
                    total_out += result.output_tokens;
                    total_cost += result.cost_usd;
                    result
                        .prompts
                        .into_iter()
                        .map(|p| (p.agent_id.clone(), p))
                        .collect()
                }
                None => HashMap::new(),
            };

        // ── Phase 2: Research (parallel per document) ────────────────────
        let research_results = self
            .execute_research_phase(
                &strategy_output.plans,
                &doc_defs,
                &model_id,
                &context_docs,
                &designed_lookup,
            )
            .await;

        for r in &research_results {
            total_in += r.input_tokens;
            total_out += r.output_tokens;
            total_cost += r.cost_usd;
        }

        let successful_research: Vec<_> = research_results
            .iter()
            .filter(|r| r.error.is_none())
            .collect();

        if successful_research.is_empty() {
            return Err(HubError::Internal(anyhow!(
                "all research phases failed for documenter step {}",
                self.step.id
            )));
        }

        if self.is_cancelled() {
            return Err(HubError::Internal(anyhow!(
                "documenter execution cancelled"
            )));
        }

        // ── Phase 3: Write (parallel per successful research) ────────────
        let write_results = self
            .execute_write_phase(
                &strategy_output.plans,
                &successful_research,
                &doc_defs,
                &model_id,
                &context_docs,
                &designed_lookup,
            )
            .await;

        for r in &write_results {
            total_in += r.input_tokens;
            total_out += r.output_tokens;
            total_cost += r.cost_usd;
        }

        // Build structured output summarising all documents
        let output_key = resolve_output_key(self.step, port_outputs);
        let doc_statuses: Vec<JsonValue> = strategy_output
            .plans
            .iter()
            .map(|plan| {
                // Check research result
                let research_ok = research_results
                    .iter()
                    .any(|r| r.document_name == plan.document_name && r.error.is_none());
                if !research_ok {
                    return serde_json::json!({
                        "name": plan.document_name,
                        "status": "failed",
                        "error": "research phase failed"
                    });
                }
                // Check write result
                match write_results
                    .iter()
                    .find(|r| r.document_name == plan.document_name)
                {
                    Some(r) if r.error.is_none() => serde_json::json!({
                        "name": plan.document_name,
                        "status": "complete"
                    }),
                    Some(r) => serde_json::json!({
                        "name": plan.document_name,
                        "status": "failed",
                        "error": r.error.as_deref().unwrap_or("write phase failed")
                    }),
                    None => serde_json::json!({
                        "name": plan.document_name,
                        "status": "failed",
                        "error": "write phase not attempted"
                    }),
                }
            })
            .collect();

        let structured = serde_json::json!({ "documents": doc_statuses });
        let raw = serde_json::to_string_pretty(&structured).unwrap_or_default();

        Ok(DocumenterResult {
            output: StepOutput {
                variable_name: output_key,
                structured_output: Some(structured),
                raw_output: raw,
            },
            input_tokens: total_in,
            output_tokens: total_out,
            cost_usd: total_cost,
        })
    }

    // ── Designer integration ─────────────────────────────────────────

    /// Run the Agent Designer for the strategist (Phase 1 pre-lifecycle).
    async fn run_designer_for_strategist(
        &self,
        doc_defs: &[crate::db::ProtocolDocumentDefRow],
    ) -> Option<super::agent_designer::DesignerResult> {
        let input = super::designer_input::documenter::build_strategist_designer_input(
            self.step,
            doc_defs,
            self.completed_envelopes,
            &[],
        );

        match super::agent_designer::run_agent_designer(
            self.engine,
            self.state,
            self.ctx,
            self.step,
            input,
            "strategist",
            self.cancel,
        )
        .await
        {
            Ok(result) => {
                info!(
                    run_id = %result.run_id,
                    "Documenter strategist designer completed"
                );
                Some(result)
            }
            Err(e) => {
                warn!("Strategist designer failed, using static prompts: {}", e);
                None
            }
        }
    }

    /// Run the Agent Designer for researchers + writers (Phase 2 & 3 pre-lifecycle).
    async fn run_designer_for_research_write(
        &self,
        plans: &[types::DocumentPlan],
    ) -> Option<super::agent_designer::DesignerResult> {
        let all_caps: Vec<String> = plans
            .iter()
            .flat_map(|p| p.required_capabilities.iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let input = super::designer_input::documenter::build_research_write_designer_input(
            self.step,
            plans,
            self.completed_envelopes,
            &all_caps,
        );

        match super::agent_designer::run_agent_designer(
            self.engine,
            self.state,
            self.ctx,
            self.step,
            input,
            "research_write",
            self.cancel,
        )
        .await
        {
            Ok(result) => {
                info!(
                    run_id = %result.run_id,
                    prompts = result.prompts.len(),
                    "Documenter research/write designer completed"
                );
                Some(result)
            }
            Err(e) => {
                warn!(
                    "Research/write designer failed, using static prompts: {}",
                    e
                );
                None
            }
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.is_some_and(|c| c.is_cancelled())
    }

    fn broadcast_phase_progress(
        &self,
        phase: &str,
        completed: usize,
        total: usize,
        document_name: Option<&str>,
    ) {
        broadcast_workflow_event(
            self.state,
            self.ctx,
            self.step.workflow_id,
            WorkflowEventKind::DocumenterPhaseProgress {
                step_id: self.step.id,
                phase: phase.to_string(),
                completed,
                total,
                document_name: document_name.map(String::from),
            },
        );
    }
}
