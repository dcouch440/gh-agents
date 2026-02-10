//! Documenter executor — phased pipeline for document generation.
//!
//! Strategy → Research → Write pipeline. When the DAG encounters a step with
//! `execution_mode = "documenter"`, it dispatches to `DocumenterExecutor`
//! which runs three phases:
//!
//! 1. **Strategy** — single-turn LLM call producing a `StrategyOutput` JSON
//! 2. **Research** — parallel per-document LLM calls with capability-resolved tools
//! 3. **Write** — parallel per-document single-turn LLM calls producing final content

use anyhow::anyhow;
use chrono::Utc;
use serde_json::Value as JsonValue;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::{ProtocolDocumentDefRow, ProtocolExecutionRow, WorkflowStepRow};
use crate::server::hub::capability_resolver::resolve_capabilities_to_tools;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::compute_cost;
use crate::server::hub::strategies::documenter_research::{
    DocumenterResearchConfig, DocumenterResearchStrategy,
};
use crate::server::hub::strategies::documenter_strategy::{
    DocumenterStrategyConfig, DocumenterStrategyStrategy,
};
use crate::server::hub::strategies::documenter_writer::{
    DocumenterWriterConfig, DocumenterWriterStrategy,
};
use crate::server::hub::streaming::NullSink;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::UserId;

use super::utils::{StepOutput, WorkflowExecutionContext};
use super::{broadcast_workflow_event, resolve_output_key};

mod tests;
pub mod types;

const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

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
pub struct DocumenterExecutor<'a> {
    engine: &'a ExecutionEngine,
    state: &'a AppState,
    ctx: &'a WorkflowExecutionContext,
    step: &'a WorkflowStepRow,
    prompt: &'a str,
    cancel: Option<&'a CancellationToken>,
}

/// Internal result from a single research or write task.
struct PhaseTaskResult {
    document_name: String,
    content: String,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
    error: Option<String>,
}

/// Internal result from the strategy phase.
struct StrategyPhaseResult {
    plans: Vec<types::DocumentPlan>,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
}

/// A context document available to the documenter pipeline.
///
/// Loaded from agent context and step documents, then selectively injected
/// into research and write phase prompts based on strategy LLM assignments.
#[derive(Debug, Clone)]
pub(crate) struct ContextDocument {
    short_id: String,
    title: String,
    content: String,
}

/// Build a `<context>` block from assigned document IDs.
///
/// - If `all_docs` is empty, returns empty string (no context exists).
/// - If `assigned_ids` is empty, includes ALL docs (backward compat).
/// - If `assigned_ids` is non-empty, filters to only matching docs.
pub(crate) fn build_context_block(assigned_ids: &[String], all_docs: &[ContextDocument]) -> String {
    if all_docs.is_empty() {
        return String::new();
    }

    let relevant_docs: Vec<&ContextDocument> = if assigned_ids.is_empty() {
        all_docs.iter().collect()
    } else {
        all_docs
            .iter()
            .filter(|d| assigned_ids.contains(&d.short_id))
            .collect()
    };

    if relevant_docs.is_empty() {
        return String::new();
    }

    let mut block = String::from("<context>");
    for doc in relevant_docs {
        block.push_str(&format!(
            "\n<document_{} title=\"{}\">\n{}\n</document_{}>",
            doc.short_id, doc.title, doc.content, doc.short_id
        ));
    }
    block.push_str("\n</context>");
    block
}

impl<'a> DocumenterExecutor<'a> {
    pub fn new(
        engine: &'a ExecutionEngine,
        state: &'a AppState,
        ctx: &'a WorkflowExecutionContext,
        step: &'a WorkflowStepRow,
        prompt: &'a str,
        cancel: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            engine,
            state,
            ctx,
            step,
            prompt,
            cancel,
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

        // Load system prompt from step protocol expansion
        let system_prompt = self.load_strategy_system_prompt().await?;

        // Determine model
        let model_id = DEFAULT_MODEL.to_string();

        // ── Phase 1: Strategy ────────────────────────────────────────────
        let strategy_output = self
            .execute_strategy_phase(&system_prompt, &model_id, &doc_defs)
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

        // ── Phase 2: Research (parallel per document) ────────────────────
        let research_results = self
            .execute_research_phase(&strategy_output.plans, &doc_defs, &model_id, &context_docs)
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
                &model_id,
                &context_docs,
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

    // ── Phase 1: Strategy ────────────────────────────────────────────────

    async fn execute_strategy_phase(
        &self,
        system_prompt: &str,
        model_id: &str,
        doc_defs: &[ProtocolDocumentDefRow],
    ) -> Result<StrategyPhaseResult, HubError> {
        info!(step_id = %self.step.id, "Documenter Phase 1: Strategy");

        // Create protocol execution record
        let exec_row = self
            .create_execution_row("strategy", None, Some(self.prompt))
            .await?;

        let strategy = DocumenterStrategyStrategy::new(DocumenterStrategyConfig {
            system_prompt: system_prompt.to_string(),
            model_id: model_id.to_string(),
            state: Some(self.state.clone()),
            user_id: Some(UserId(self.ctx.user_id)),
        });

        let recorder = ExecutionRecorder::new(&**self.state.repo(), None, None);
        let result = self
            .engine
            .execute(&strategy, self.prompt, &NullSink, &recorder, self.cancel)
            .await;

        match result {
            Ok(exec_result) => {
                let cost = compute_cost(
                    model_id,
                    exec_result.input_tokens as i64,
                    exec_result.output_tokens as i64,
                );

                // Parse strategy output
                match serde_json::from_str::<types::StrategyOutput>(&exec_result.content) {
                    Ok(strategy_output) => {
                        // Validate: every plan should reference a known doc def
                        let def_names: Vec<&str> =
                            doc_defs.iter().map(|d| d.name.as_str()).collect();
                        for plan in &strategy_output.document_plans {
                            if !def_names.contains(&plan.document_name.as_str()) {
                                warn!(
                                    doc = %plan.document_name,
                                    "Strategy LLM referenced unknown document name"
                                );
                            }
                        }

                        self.update_execution_row(
                            exec_row.id,
                            "complete",
                            Some(&exec_result.content),
                            None,
                            exec_result.input_tokens as i64,
                            exec_result.output_tokens as i64,
                            cost,
                            Some(model_id),
                        )
                        .await;

                        self.broadcast_phase_progress("strategy", 1, 1, None);

                        Ok(StrategyPhaseResult {
                            plans: strategy_output.document_plans,
                            input_tokens: exec_result.input_tokens as i64,
                            output_tokens: exec_result.output_tokens as i64,
                            cost_usd: cost,
                        })
                    }
                    Err(parse_err) => {
                        let err_msg = format!("Failed to parse strategy output: {}", parse_err);
                        error!(step_id = %self.step.id, %err_msg);
                        self.update_execution_row(
                            exec_row.id,
                            "failed",
                            Some(&exec_result.content),
                            Some(&err_msg),
                            exec_result.input_tokens as i64,
                            exec_result.output_tokens as i64,
                            cost,
                            Some(model_id),
                        )
                        .await;
                        Err(HubError::Internal(anyhow!(err_msg)))
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("Strategy phase LLM call failed: {}", e);
                error!(step_id = %self.step.id, %err_msg);
                self.update_execution_row(
                    exec_row.id,
                    "failed",
                    None,
                    Some(&err_msg),
                    0,
                    0,
                    0.0,
                    None,
                )
                .await;
                Err(HubError::Internal(anyhow!(err_msg)))
            }
        }
    }

    // ── Phase 2: Research ────────────────────────────────────────────────

    async fn execute_research_phase(
        &self,
        plans: &[types::DocumentPlan],
        doc_defs: &[ProtocolDocumentDefRow],
        model_id: &str,
        context_docs: &[ContextDocument],
    ) -> Vec<PhaseTaskResult> {
        info!(
            step_id = %self.step.id,
            count = plans.len(),
            "Documenter Phase 2: Research"
        );

        let total = plans.len();
        let mut join_set = JoinSet::new();

        for plan in plans {
            let doc_def = doc_defs.iter().find(|d| d.name == plan.document_name);
            let doc_def_id = doc_def.map(|d| d.id);

            // Create execution row before spawning
            let exec_row = match self
                .create_execution_row("research", doc_def_id, Some(&plan.research_strategy))
                .await
            {
                Ok(row) => row,
                Err(e) => {
                    warn!(doc = %plan.document_name, "Failed to create research execution row: {}", e);
                    continue;
                }
            };

            // Build context block for this plan's assigned documents
            let context_block =
                build_context_block(&plan.context_document_ids, context_docs);

            // Clone everything needed for the spawned task
            let engine = self.engine.clone_with_provider();
            let state = self.state.clone();
            let user_id = self.ctx.user_id;
            let execution_context = self.ctx.execution_context.clone();
            let model = model_id.to_string();
            let doc_name = plan.document_name.clone();
            let research_prompt = if context_block.is_empty() {
                plan.research_strategy.clone()
            } else {
                format!("{}\n\n{}", plan.research_strategy, context_block)
            };
            let capabilities = plan.required_capabilities.clone();
            let _exec_id = exec_row.id;

            join_set.spawn(async move {
                // Resolve capabilities to tools
                let (tools, tool_names) = match resolve_capabilities_to_tools(
                    &capabilities,
                    &*state.repos().tool_capabilities,
                )
                .await
                {
                    Ok(resolved) => resolved,
                    Err(e) => {
                        warn!(doc = %doc_name, "Capability resolution failed: {}", e);
                        (vec![], vec![])
                    }
                };

                let system_prompt = format!(
                    "You are a research assistant gathering information for a document titled \"{}\".\n\
                     Use the available tools to gather comprehensive, accurate information.\n\
                     Summarize your findings clearly — your output will be used by a writer to produce the final document.",
                    doc_name
                );

                let strategy = DocumenterResearchStrategy::new(DocumenterResearchConfig {
                    system_prompt,
                    model_id: model.clone(),
                    tools,
                    tool_names,
                    execution_context,
                    state: Some(state.clone()),
                    user_id: Some(UserId(user_id)),
                });

                let recorder = ExecutionRecorder::new(&**state.repo(), None, None);
                let result = engine
                    .execute(&strategy, &research_prompt, &NullSink, &recorder, None)
                    .await;

                match result {
                    Ok(exec_result) => {
                        let cost = compute_cost(
                            &model,
                            exec_result.input_tokens as i64,
                            exec_result.output_tokens as i64,
                        );
                        PhaseTaskResult {
                            document_name: doc_name,
                            content: exec_result.content,
                            input_tokens: exec_result.input_tokens as i64,
                            output_tokens: exec_result.output_tokens as i64,
                            cost_usd: cost,
                            error: None,
                        }
                    }
                    Err(e) => PhaseTaskResult {
                        document_name: doc_name,
                        content: String::new(),
                        input_tokens: 0,
                        output_tokens: 0,
                        cost_usd: 0.0,
                        error: Some(format!("{}", e)),
                    },
                }
            });
        }

        // Collect results
        let mut results = Vec::new();
        let mut completed_count = 0;

        while let Some(join_result) = join_set.join_next().await {
            completed_count += 1;
            match join_result {
                Ok(task_result) => {
                    let status = if task_result.error.is_none() {
                        "complete"
                    } else {
                        "failed"
                    };
                    // We can't easily map back to exec_id here without storing it,
                    // so we broadcast progress only
                    self.broadcast_phase_progress(
                        "research",
                        completed_count,
                        total,
                        Some(&task_result.document_name),
                    );
                    if let Some(ref err) = task_result.error {
                        warn!(
                            doc = %task_result.document_name,
                            %status,
                            "Research failed: {}", err
                        );
                    } else {
                        info!(
                            doc = %task_result.document_name,
                            tokens_in = task_result.input_tokens,
                            tokens_out = task_result.output_tokens,
                            "Research completed"
                        );
                    }
                    results.push(task_result);
                }
                Err(join_err) => {
                    error!("Research task panicked: {}", join_err);
                    completed_count += 0; // already incremented
                }
            }
        }

        results
    }

    // ── Phase 3: Write ───────────────────────────────────────────────────

    async fn execute_write_phase(
        &self,
        plans: &[types::DocumentPlan],
        successful_research: &[&PhaseTaskResult],
        model_id: &str,
        context_docs: &[ContextDocument],
    ) -> Vec<PhaseTaskResult> {
        info!(
            step_id = %self.step.id,
            count = successful_research.len(),
            "Documenter Phase 3: Write"
        );

        let total = successful_research.len();
        let mut join_set = JoinSet::new();

        // Load doc defs again to get document_id for each
        let doc_defs = self
            .state
            .repos()
            .workflows
            .list_document_defs(self.step.id)
            .await
            .unwrap_or_default();

        for research in successful_research {
            let plan = plans
                .iter()
                .find(|p| p.document_name == research.document_name);
            let writer_prompt_prefix = plan
                .map(|p| p.writer_prompt.as_str())
                .unwrap_or("Write the document based on the research findings below.");

            let doc_def = doc_defs.iter().find(|d| d.name == research.document_name);
            let doc_def_id = doc_def.map(|d| d.id);
            let document_id = doc_def.and_then(|d| d.document_id);

            // Build context block for this plan's assigned documents
            let context_ids = plan
                .map(|p| p.context_document_ids.as_slice())
                .unwrap_or(&[]);
            let context_block = build_context_block(context_ids, context_docs);

            // Create execution row
            let input_prompt = if context_block.is_empty() {
                format!(
                    "{}\n\n---\n\nResearch findings:\n{}",
                    writer_prompt_prefix, research.content
                )
            } else {
                format!(
                    "{}\n\n{}\n\n---\n\nResearch findings:\n{}",
                    writer_prompt_prefix, context_block, research.content
                )
            };
            let exec_row = match self
                .create_execution_row("write", doc_def_id, Some(&input_prompt))
                .await
            {
                Ok(row) => row,
                Err(e) => {
                    warn!(doc = %research.document_name, "Failed to create write execution row: {}", e);
                    continue;
                }
            };

            // Clone for spawned task
            let engine = self.engine.clone_with_provider();
            let state = self.state.clone();
            let user_id = self.ctx.user_id;
            let model = model_id.to_string();
            let doc_name = research.document_name.clone();
            let prompt = input_prompt;
            let _exec_id = exec_row.id;
            let doc_id = document_id;

            join_set.spawn(async move {
                let system_prompt = format!(
                    "You are a technical writer. Produce a well-structured, comprehensive document \
                     titled \"{}\". Write in clear, professional prose. Use markdown formatting.",
                    doc_name
                );

                let strategy = DocumenterWriterStrategy::new(DocumenterWriterConfig {
                    system_prompt,
                    model_id: model.clone(),
                    state: Some(state.clone()),
                    user_id: Some(UserId(user_id)),
                });

                let recorder = ExecutionRecorder::new(&**state.repo(), None, None);
                let result = engine
                    .execute(&strategy, &prompt, &NullSink, &recorder, None)
                    .await;

                match result {
                    Ok(exec_result) => {
                        let cost = compute_cost(
                            &model,
                            exec_result.input_tokens as i64,
                            exec_result.output_tokens as i64,
                        );

                        // Update document content if we have a document_id
                        if let Some(did) = doc_id {
                            if let Some(doc_repo) = state.doc_repo() {
                                let _ = doc_repo
                                    .update_document(
                                        did,
                                        Some(exec_result.content.clone()),
                                        None,
                                        None,
                                    )
                                    .await;
                            }
                        }

                        PhaseTaskResult {
                            document_name: doc_name,
                            content: exec_result.content,
                            input_tokens: exec_result.input_tokens as i64,
                            output_tokens: exec_result.output_tokens as i64,
                            cost_usd: cost,
                            error: None,
                        }
                    }
                    Err(e) => PhaseTaskResult {
                        document_name: doc_name,
                        content: String::new(),
                        input_tokens: 0,
                        output_tokens: 0,
                        cost_usd: 0.0,
                        error: Some(format!("{}", e)),
                    },
                }
            });
        }

        // Collect results
        let mut results = Vec::new();
        let mut completed_count = 0;

        while let Some(join_result) = join_set.join_next().await {
            completed_count += 1;
            match join_result {
                Ok(task_result) => {
                    self.broadcast_phase_progress(
                        "write",
                        completed_count,
                        total,
                        Some(&task_result.document_name),
                    );
                    if let Some(ref err) = task_result.error {
                        warn!(doc = %task_result.document_name, "Write failed: {}", err);
                    } else {
                        info!(
                            doc = %task_result.document_name,
                            tokens_in = task_result.input_tokens,
                            tokens_out = task_result.output_tokens,
                            "Write completed"
                        );
                    }
                    results.push(task_result);
                }
                Err(join_err) => {
                    error!("Write task panicked: {}", join_err);
                }
            }
        }

        results
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    /// Load the strategy system prompt from the step's protocol expansion.
    ///
    /// Falls back to generating a basic prompt from doc defs if no expansion is found.
    async fn load_strategy_system_prompt(&self) -> Result<String, HubError> {
        // Try to load from step protocol expansion
        if let Ok(Some(step_protocol)) = self
            .state
            .repos()
            .protocols
            .get_step_protocol(self.step.id)
            .await
        {
            if let Some(prompt) = step_protocol
                .applied_expansion
                .get("prompt_injection")
                .and_then(|v| v.as_str())
            {
                return Ok(prompt.to_string());
            }
        }

        // Fallback: generate from doc defs
        let doc_defs = self
            .state
            .repos()
            .workflows
            .list_document_defs(self.step.id)
            .await
            .map_err(|e| HubError::Internal(anyhow!("failed to load doc defs: {}", e)))?;

        let doc_values: Vec<JsonValue> = doc_defs
            .iter()
            .map(|d| {
                serde_json::json!({
                    "name": d.name,
                    "description": d.description,
                    "target_length": d.target_length,
                })
            })
            .collect();

        Ok(crate::server::hub::protocols::prompt_gen::documenter_prompt(&doc_values, &[], true))
    }

    /// Load context documents from agent context and step documents.
    ///
    /// Mirrors the document fetching in `compose_prompt()`, returning structured
    /// data that can be selectively injected into research and write phase prompts.
    async fn load_context_documents(&self) -> Vec<ContextDocument> {
        let mut docs = Vec::new();

        // Agent context documents
        if let Some(agent_id) = self.step.agent_id {
            if let Ok(agent_docs) = self.state.repo().get_agent_context(agent_id).await {
                for doc in agent_docs {
                    let short_id = doc.id.to_string()[..8].to_string();
                    docs.push(ContextDocument {
                        short_id,
                        title: doc.title,
                        content: doc.content,
                    });
                }
            }
        }

        // Step documents
        if let Ok(step_docs) = self
            .state
            .repos()
            .workflows
            .list_step_documents(self.step.id)
            .await
        {
            if let Some(d_repo) = self.state.doc_repo() {
                for sd in &step_docs {
                    if let Ok(Some(doc)) = d_repo.get_document(sd.document_id).await {
                        let short_id = doc.id.to_string()[..8].to_string();
                        docs.push(ContextDocument {
                            short_id,
                            title: doc.title,
                            content: doc.content,
                        });
                    }
                }
            }
        }

        docs
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

    async fn create_execution_row(
        &self,
        phase: &str,
        document_def_id: Option<Uuid>,
        input_prompt: Option<&str>,
    ) -> Result<ProtocolExecutionRow, HubError> {
        let row = ProtocolExecutionRow {
            id: Uuid::new_v4(),
            protocol_step_id: self.step.id,
            workflow_run_id: Some(self.ctx.run_id),
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
        };

        self.state
            .repos()
            .protocols
            .create_protocol_execution(row)
            .await
            .map_err(|e| HubError::Internal(anyhow!("failed to create execution row: {}", e)))
    }

    async fn update_execution_row(
        &self,
        id: Uuid,
        status: &str,
        output_content: Option<&str>,
        error_message: Option<&str>,
        tokens_in: i64,
        tokens_out: i64,
        cost_usd: f32,
        model: Option<&str>,
    ) {
        let _ = self
            .state
            .repos()
            .protocols
            .update_protocol_execution_status(
                id,
                status.to_string(),
                output_content.map(String::from),
                error_message.map(String::from),
                Some(tokens_in as i32),
                Some(tokens_out as i32),
                Some(cost_usd as f64),
                model.map(String::from),
            )
            .await;
    }
}

/// Build a structured output JSON summarising document results.
///
/// Used by tests and the executor to create the final `StepOutput`.
pub fn build_documents_output(statuses: Vec<JsonValue>) -> JsonValue {
    serde_json::json!({ "documents": statuses })
}
