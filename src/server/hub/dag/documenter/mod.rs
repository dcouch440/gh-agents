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
use serde_json::Value as JsonValue;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::{ProtocolDocumentDefRow, WorkflowStepRow};
use crate::server::hub::capability_resolver::resolve_capabilities_to_tools;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::execution_recorder::ProtocolExecutionRecorder;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::compute_cost;
use crate::server::hub::strategies::documenter::coordinator::{
    DocumenterCoordinatorConfig, DocumenterCoordinatorStrategy,
};
use crate::server::hub::strategies::documenter::research::{
    DocumenterResearchConfig, DocumenterResearchStrategy,
};
use crate::server::hub::strategies::documenter::writer::{
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
pub(crate) struct DocumenterExecutor<'a> {
    engine: &'a ExecutionEngine,
    state: &'a AppState,
    ctx: &'a WorkflowExecutionContext,
    step: &'a WorkflowStepRow,
    prompt: &'a str,
    cancel: Option<&'a CancellationToken>,
    upstream_context: &'a [ContextDocument],
    recorder: ProtocolExecutionRecorder<'a>,
}

/// Internal result from a single research or write task.
struct PhaseTaskResult {
    exec_id: Uuid,
    document_name: String,
    content: String,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
    model: String,
    error: Option<String>,
}

/// Internal result from the strategy phase.
struct StrategyPhaseResult {
    plans: Vec<types::DocumentPlan>,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
}

pub(crate) use crate::server::hub::protocols::context::{build_context_block, ContextDocument};
use crate::server::hub::protocols::json_utils::extract_json_from_llm_response;

impl<'a> DocumenterExecutor<'a> {
    pub fn new(
        engine: &'a ExecutionEngine,
        state: &'a AppState,
        ctx: &'a WorkflowExecutionContext,
        step: &'a WorkflowStepRow,
        prompt: &'a str,
        cancel: Option<&'a CancellationToken>,
        upstream_context: &'a [ContextDocument],
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
            .recorder
            .create_phase("strategy", None, Some(self.prompt))
            .await?;

        let strategy = DocumenterCoordinatorStrategy::new(DocumenterCoordinatorConfig {
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

                // Parse strategy output (strip markdown code fences if present)
                let json_str = extract_json_from_llm_response(&exec_result.content);
                match serde_json::from_str::<types::StrategyOutput>(&json_str) {
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

                        self.recorder
                            .update_phase(
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
                        self.recorder
                            .update_phase(
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
                self.recorder
                    .update_phase(exec_row.id, "failed", None, Some(&err_msg), 0, 0, 0.0, None)
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
                .recorder
                .create_phase("research", doc_def_id, Some(&plan.research_strategy))
                .await
            {
                Ok(row) => row,
                Err(e) => {
                    warn!(doc = %plan.document_name, "Failed to create research execution row: {}", e);
                    continue;
                }
            };

            // Build context block for this plan's assigned documents
            let context_block = build_context_block(&plan.context_document_ids, context_docs);

            // Clone everything needed for the spawned task
            let engine = self.engine.clone_with_provider();
            let state = self.state.clone();
            let user_id = self.ctx.user_id;
            let execution_context = self.ctx.execution_context.clone();
            let model = model_id.to_string();
            let doc_name = plan.document_name.clone();
            let research_prompt = compose_research_prompt(&plan.research_strategy, &context_block);
            let capabilities = plan.required_capabilities.clone();
            let exec_id = exec_row.id;

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
                            exec_id,
                            document_name: doc_name,
                            content: exec_result.content,
                            input_tokens: exec_result.input_tokens as i64,
                            output_tokens: exec_result.output_tokens as i64,
                            cost_usd: cost,
                            model,
                            error: None,
                        }
                    }
                    Err(e) => PhaseTaskResult {
                        exec_id,
                        document_name: doc_name,
                        content: String::new(),
                        input_tokens: 0,
                        output_tokens: 0,
                        cost_usd: 0.0,
                        model,
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
                    self.recorder
                        .update_phase(
                            task_result.exec_id,
                            status,
                            Some(&task_result.content),
                            task_result.error.as_deref(),
                            task_result.input_tokens,
                            task_result.output_tokens,
                            task_result.cost_usd,
                            Some(&task_result.model),
                        )
                        .await;
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
            let input_prompt =
                compose_write_prompt(writer_prompt_prefix, &context_block, &research.content);
            let exec_row = match self
                .recorder
                .create_phase("write", doc_def_id, Some(&input_prompt))
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
            let exec_id = exec_row.id;
            let doc_id = document_id;
            let def_id = doc_def_id;
            let workflow_id = self.step.workflow_id;
            let step_id = self.step.id;

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

                        // Persist document content
                        match determine_persist_action(doc_id, def_id) {
                            DocumentPersistAction::Update(did) => {
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
                            DocumentPersistAction::CreateAndLink(did) => {
                                if let Some(doc_repo) = state.doc_repo() {
                                    match doc_repo
                                        .create_workflow_document(
                                            user_id,
                                            doc_name.clone(),
                                            workflow_id,
                                            None,
                                            Some(step_id),
                                        )
                                        .await
                                    {
                                        Ok(doc) => {
                                            let _ = state
                                                .repos()
                                                .workflows
                                                .link_document_to_def(did, doc.id)
                                                .await;
                                            let _ = doc_repo
                                                .update_document(
                                                    doc.id,
                                                    Some(exec_result.content.clone()),
                                                    None,
                                                    None,
                                                )
                                                .await;
                                        }
                                        Err(e) => {
                                            warn!(doc = %doc_name, "Failed to create document: {}", e);
                                        }
                                    }
                                }
                            }
                            DocumentPersistAction::Skip => {}
                        }

                        PhaseTaskResult {
                            exec_id,
                            document_name: doc_name,
                            content: exec_result.content,
                            input_tokens: exec_result.input_tokens as i64,
                            output_tokens: exec_result.output_tokens as i64,
                            cost_usd: cost,
                            model,
                            error: None,
                        }
                    }
                    Err(e) => PhaseTaskResult {
                        exec_id,
                        document_name: doc_name,
                        content: String::new(),
                        input_tokens: 0,
                        output_tokens: 0,
                        cost_usd: 0.0,
                        model,
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
                    self.recorder
                        .update_phase(
                            task_result.exec_id,
                            status,
                            Some(&task_result.content),
                            task_result.error.as_deref(),
                            task_result.input_tokens,
                            task_result.output_tokens,
                            task_result.cost_usd,
                            Some(&task_result.model),
                        )
                        .await;
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

        Ok(
            crate::server::hub::protocols::compilers::documenter::prompt::documenter_prompt(
                &doc_values,
                &[],
                true,
            ),
        )
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

        // Upstream context from completed context-mode steps connected by bare edges
        // (pre-built with stable short_ids by the caller)
        docs.extend(self.upstream_context.iter().cloned());

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
}

/// Build a structured output JSON summarising document results.
///
/// Used by tests and the executor to create the final `StepOutput`.
pub fn build_documents_output(statuses: Vec<JsonValue>) -> JsonValue {
    serde_json::json!({ "documents": statuses })
}

/// What the write phase should do with the generated document content.
#[derive(Debug, PartialEq)]
pub(crate) enum DocumentPersistAction {
    /// A document already exists — update it in place.
    Update(Uuid),
    /// No document exists but a def is available — create a new document and link it.
    CreateAndLink(Uuid),
    /// Neither document nor def available — content cannot be persisted.
    Skip,
}

/// Determine how to persist a write phase result based on the current state
/// of the document definition.
///
/// - `document_id` — the existing linked document (from `protocol_document_defs.document_id`)
/// - `def_id` — the document definition row id (for linking a newly created document)
pub(crate) fn determine_persist_action(
    document_id: Option<Uuid>,
    def_id: Option<Uuid>,
) -> DocumentPersistAction {
    if let Some(did) = document_id {
        DocumentPersistAction::Update(did)
    } else if let Some(did) = def_id {
        DocumentPersistAction::CreateAndLink(did)
    } else {
        DocumentPersistAction::Skip
    }
}

/// Compose the user prompt for a write phase LLM call.
///
/// Combines the writer instructions (from strategy LLM), optional context
/// documents, and research findings into a single prompt.
pub(crate) fn compose_write_prompt(
    writer_prompt: &str,
    context_block: &str,
    research_content: &str,
) -> String {
    if context_block.is_empty() {
        format!(
            "{}\n\n---\n\nResearch findings:\n{}",
            writer_prompt, research_content
        )
    } else {
        format!(
            "{}\n\n{}\n\n---\n\nResearch findings:\n{}",
            writer_prompt, context_block, research_content
        )
    }
}

/// Compose the user prompt for a research phase LLM call.
///
/// Combines the research strategy (from strategy LLM) with optional context
/// documents.
pub(crate) fn compose_research_prompt(research_strategy: &str, context_block: &str) -> String {
    if context_block.is_empty() {
        research_strategy.to_string()
    } else {
        format!("{}\n\n{}", research_strategy, context_block)
    }
}
