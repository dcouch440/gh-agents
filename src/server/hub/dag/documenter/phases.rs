//! Phase execution methods for the documenter pipeline.
//!
//! Contains the strategy, research, and write phase implementations as a split
//! `impl DocumenterExecutor` block, plus shared phase result collection logic.

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tokio::task::JoinSet;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::ProtocolDocumentDefRow;
use crate::server::hub::capability_resolver::resolve_capabilities_to_tools;
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::json_utils::extract_json_from_llm_response;
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
use crate::types::UserId;

use super::persistence::{persist_document_content, DocumentPersistContext};
use super::prompts::{
    build_research_system_prompt, build_writer_system_prompt, compose_research_prompt,
    compose_write_prompt,
};
use super::types;
use super::{ContextDocument, DocumenterExecutor};

// ── Phase result types ───────────────────────────────────────────────────────

/// Internal result from a single research or write task.
pub(super) struct PhaseTaskResult {
    pub exec_id: Uuid,
    pub document_name: String,
    pub content: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    pub model: String,
    pub error: Option<String>,
}

/// Internal result from the strategy phase.
pub(super) struct StrategyPhaseResult {
    pub plans: Vec<types::DocumentPlan>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
}

// ── Phase implementations ────────────────────────────────────────────────────

impl<'a> DocumenterExecutor<'a> {
    // ── Phase 1: Strategy ────────────────────────────────────────────────

    pub(super) async fn execute_strategy_phase(
        &self,
        system_prompt: &str,
        model_id: &str,
        doc_defs: &[ProtocolDocumentDefRow],
    ) -> Result<StrategyPhaseResult, HubError> {
        info!(step_id = %self.step.id, "Documenter Phase 1: Strategy");

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

                let json_str = extract_json_from_llm_response(&exec_result.content);
                match serde_json::from_str::<types::StrategyOutput>(&json_str) {
                    Ok(strategy_output) => {
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

    pub(super) async fn execute_research_phase(
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

        let doc_def_map: HashMap<&str, &ProtocolDocumentDefRow> =
            doc_defs.iter().map(|d| (d.name.as_str(), d)).collect();

        for plan in plans {
            let doc_def = doc_def_map.get(plan.document_name.as_str()).copied();
            let doc_def_id = doc_def.map(|d| d.id);

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

            let context_block =
                super::build_context_block(&plan.context_document_ids, context_docs);

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

                let system_prompt = build_research_system_prompt(&doc_name);

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

        self.collect_phase_results(join_set, "research", total)
            .await
    }

    // ── Phase 3: Write ───────────────────────────────────────────────────

    pub(super) async fn execute_write_phase(
        &self,
        plans: &[types::DocumentPlan],
        successful_research: &[&PhaseTaskResult],
        doc_defs: &[ProtocolDocumentDefRow],
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

        let doc_def_map: HashMap<&str, &ProtocolDocumentDefRow> =
            doc_defs.iter().map(|d| (d.name.as_str(), d)).collect();
        let plan_map: HashMap<&str, &types::DocumentPlan> = plans
            .iter()
            .map(|p| (p.document_name.as_str(), p))
            .collect();

        for research in successful_research {
            let plan = plan_map.get(research.document_name.as_str()).copied();
            let writer_prompt_prefix = plan
                .map(|p| p.writer_prompt.as_str())
                .unwrap_or("Write the document based on the research findings below.");

            let doc_def = doc_def_map.get(research.document_name.as_str()).copied();
            let doc_def_id = doc_def.map(|d| d.id);
            let document_id = doc_def.and_then(|d| d.document_id);

            let context_ids = plan
                .map(|p| p.context_document_ids.as_slice())
                .unwrap_or(&[]);
            let context_block = super::build_context_block(context_ids, context_docs);

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
                let system_prompt = build_writer_system_prompt(&doc_name);

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

                        let persist_ctx = DocumentPersistContext {
                            document_id: doc_id,
                            def_id,
                            doc_name: doc_name.clone(),
                            user_id,
                            workflow_id,
                            step_id,
                        };
                        persist_document_content(&state, &persist_ctx, &exec_result.content).await;

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

        self.collect_phase_results(join_set, "write", total).await
    }

    // ── Shared result collection ─────────────────────────────────────────

    /// Collect results from a set of spawned phase tasks, recording each result
    /// and broadcasting progress.
    pub(super) async fn collect_phase_results(
        &self,
        mut join_set: JoinSet<PhaseTaskResult>,
        phase: &str,
        total: usize,
    ) -> Vec<PhaseTaskResult> {
        let mut results = Vec::with_capacity(join_set.len());
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
                        phase,
                        completed_count,
                        total,
                        Some(&task_result.document_name),
                    );
                    if let Some(ref err) = task_result.error {
                        warn!(
                            doc = %task_result.document_name,
                            %status,
                            "{} failed: {}", phase, err
                        );
                    } else {
                        info!(
                            doc = %task_result.document_name,
                            tokens_in = task_result.input_tokens,
                            tokens_out = task_result.output_tokens,
                            "{} completed", phase
                        );
                    }
                    results.push(task_result);
                }
                Err(join_err) => {
                    error!("{} task panicked: {}", phase, join_err);
                }
            }
        }

        results
    }

    // ── Context loading ──────────────────────────────────────────────────

    /// Load the strategy system prompt from the step's protocol expansion.
    ///
    /// Falls back to generating a basic prompt from doc defs if no expansion is found.
    pub(super) async fn load_strategy_system_prompt(&self) -> Result<String, HubError> {
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
    pub(super) async fn load_context_documents(&self) -> Vec<ContextDocument> {
        let mut docs = Vec::new();

        if let Some(agent_id) = self.step.agent_id {
            if let Ok(agent_docs) = self.state.repo().get_agent_context(agent_id).await {
                for doc in agent_docs {
                    let full_id = doc.id.to_string();
                    let short_id = full_id[..8].to_owned();
                    docs.push(ContextDocument {
                        short_id,
                        title: doc.title,
                        content: doc.content,
                    });
                }
            }
        }

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
                        let full_id = doc.id.to_string();
                        let short_id = full_id[..8].to_owned();
                        docs.push(ContextDocument {
                            short_id,
                            title: doc.title,
                            content: doc.content,
                        });
                    }
                }
            }
        }

        docs.extend(self.upstream_context.iter().cloned());

        docs
    }
}
