//! Belief capture step execution within the DAG.
//!
//! When the DAG encounters a step with `execution_mode = "belief_capture"`, this
//! module loads the extraction plan, collects all upstream content, runs one LLM
//! extraction call per source, parses structured beliefs, applies confidence
//! filtering, and stores beliefs in the DB.

mod normalize;
mod tests;

use std::collections::HashMap;

use anyhow::anyhow;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::protocols::{roles, vars, BELIEF_CAPTURE};
use crate::db::{BeliefExtractionPlanRow, BeliefRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::execution_recorder::ProtocolExecutionRecorder;
use crate::server::hub::protocols::json_utils::parse_structured_output;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::belief_capture::{
    BeliefCaptureExtractorConfig, BeliefCaptureExtractorStrategy,
};
use crate::server::hub::strategies::compute_cost;
use crate::server::hub::streaming::NullSink;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope, UserId};

use super::{
    broadcast_workflow_event, resolve_output_key, step_display_name, PortMetadata, StepOutput,
    WorkflowExecutionContext,
};

/// Execute a belief capture step within the DAG.
///
/// Loads the extraction plan, collects upstream content, runs per-source
/// LLM extraction, applies confidence filtering, and stores beliefs in DB.
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_belief_capture_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    var_outputs: &mut HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    completed_envelopes: &mut HashMap<Uuid, StepExecutionEnvelope>,
    port_meta: &PortMetadata,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    let step_start = std::time::Instant::now();

    // 1. Broadcast step started
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            execution_id: None,
        },
    );

    // 2. Load extraction plan (use defaults if none configured)
    let plan = state
        .repos()
        .workflows
        .get_extraction_plan(step.id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load extraction plan: {}", e)))?;

    let plan = plan.unwrap_or_else(|| BeliefExtractionPlanRow {
        id: Uuid::new_v4(),
        step_id: step.id,
        extraction_focus: String::new(),
        tag_vocabulary: vec![],
        contradiction_handling: "flag".to_string(),
        confidence_threshold: "low".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    // 3. Collect upstream sources
    let sources =
        normalize::collect_upstream_sources(step, edges, steps, completed_envelopes, state).await;

    if sources.is_empty() {
        return Err(HubError::Internal(anyhow!(
            "belief_capture step {} has no upstream content to extract from",
            step.id
        )));
    }

    info!(
        step_id = %step.id,
        sources = sources.len(),
        extraction_focus = %plan.extraction_focus,
        "Starting belief capture step execution"
    );

    // 4. Create protocol execution recorder
    let recorder = ProtocolExecutionRecorder::new(&*state.repos().protocols, step.id, ctx.run_id);

    let extractor_cfg = BELIEF_CAPTURE.agent("extractor");
    let total_sources = sources.len();
    let mut step_in_tokens: i64 = 0;
    let mut step_out_tokens: i64 = 0;
    let mut step_cost: f32 = 0.0;
    let mut all_beliefs: Vec<BeliefRow> = Vec::new();

    // 5. Per-source extraction loop
    for (idx, source) in sources.iter().enumerate() {
        // Check cancellation
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        // Broadcast progress
        broadcast_workflow_event(
            state,
            ctx,
            step.workflow_id,
            WorkflowEventKind::BeliefExtractionProgress {
                step_id: step.id,
                source_step_name: source.source_step_name.clone(),
                sources_completed: idx,
                sources_total: total_sources,
                beliefs_extracted: all_beliefs.len(),
            },
        );

        // Create protocol execution row
        let exec_row = recorder
            .create_phase(
                &format!("extract_{}", idx),
                None,
                Some(&format!("Source: {}", source.title)),
            )
            .await?;

        // Build template variables
        let tag_vocab_str = if plan.tag_vocabulary.is_empty() {
            "No specific vocabulary specified — use your best judgment.".to_string()
        } else {
            plan.tag_vocabulary.join(", ")
        };

        let template_vars = HashMap::from([
            (
                vars::belief_capture::EXTRACTION_FOCUS.to_string(),
                if plan.extraction_focus.is_empty() {
                    "Extract all meaningful beliefs, claims, and insights.".to_string()
                } else {
                    plan.extraction_focus.clone()
                },
            ),
            (
                vars::belief_capture::TAG_VOCABULARY.to_string(),
                tag_vocab_str,
            ),
            (
                vars::belief_capture::CONTRADICTION_HANDLING.to_string(),
                plan.contradiction_handling.clone(),
            ),
            (
                vars::belief_capture::SOURCE_STEP_NAME.to_string(),
                source.title.clone(),
            ),
            (
                vars::belief_capture::SOURCE_TYPE.to_string(),
                source.source_type.clone(),
            ),
            (
                vars::belief_capture::SOURCE_CONTENT.to_string(),
                source.content.clone(),
            ),
        ]);
        let protocol_ctx = roles::BELIEF_CAPTURE_EXTRACTOR.resolve(&template_vars);

        // Build strategy
        let strategy = BeliefCaptureExtractorStrategy::new(BeliefCaptureExtractorConfig {
            system_prompt: protocol_ctx.system_prompt,
            model_id: extractor_cfg.model_id.clone(),
            temperature: extractor_cfg.temperature,
            max_rounds: extractor_cfg.max_rounds,
            context_budget: extractor_cfg.context_budget,
            state: Some(state.clone()),
            user_id: Some(UserId(ctx.user_id)),
        });

        // Execute via engine
        let inner_recorder = ExecutionRecorder::new(&**state.repo(), None, None);
        let result = engine
            .clone_with_provider()
            .execute(
                &strategy,
                &protocol_ctx.user_prompt,
                &NullSink,
                &inner_recorder,
                cancel,
            )
            .await;

        match result {
            Ok(exec_result) => {
                let cost = compute_cost(
                    &extractor_cfg.model_id,
                    exec_result.input_tokens as i64,
                    exec_result.output_tokens as i64,
                );
                step_in_tokens += exec_result.input_tokens as i64;
                step_out_tokens += exec_result.output_tokens as i64;
                step_cost += cost;

                // Parse structured output
                let extracted = parse_extraction_output(&exec_result.content);
                let mut source_belief_count = 0;

                for extracted_belief in extracted {
                    // Apply confidence filtering
                    if !confidence_meets_threshold(
                        &extracted_belief.confidence,
                        &plan.confidence_threshold,
                    ) {
                        continue;
                    }

                    let belief_row = BeliefRow {
                        id: Uuid::new_v4(),
                        workflow_id: step.workflow_id,
                        workflow_execution_id: Some(ctx.run_id),
                        source_step_id: source.source_step_id,
                        source_document_title: source.source_document_title.clone(),
                        source_document_def_id: source.source_document_def_id,
                        source_phase: "execution".to_string(),
                        content: extracted_belief.content,
                        reasoning: extracted_belief.reasoning,
                        belief_type: extracted_belief.belief_type,
                        confidence: extracted_belief.confidence,
                        confidence_justification: extracted_belief.confidence_justification,
                        semantic_tags: extracted_belief.semantic_tags,
                        emotional_tone: extracted_belief.emotional_tone,
                        cross_source_tension: extracted_belief.cross_source_tension,
                        source_step_name: source.source_step_name.clone(),
                        extraction_model: extractor_cfg.model_id.clone(),
                        extraction_tokens_in: exec_result.input_tokens as i32,
                        extraction_tokens_out: exec_result.output_tokens as i32,
                        created_at: Utc::now(),
                    };

                    if let Err(e) = state.repos().workflows.insert_belief(&belief_row).await {
                        warn!(
                            step_id = %step.id,
                            source = %source.title,
                            "Failed to insert belief: {}", e
                        );
                    } else {
                        source_belief_count += 1;
                        all_beliefs.push(belief_row);
                    }
                }

                recorder
                    .update_phase(
                        exec_row.id,
                        "complete",
                        Some(&exec_result.content),
                        None,
                        exec_result.input_tokens as i64,
                        exec_result.output_tokens as i64,
                        cost,
                        Some(&extractor_cfg.model_id),
                    )
                    .await;

                info!(
                    source = %source.title,
                    idx = idx,
                    beliefs = source_belief_count,
                    tokens_in = exec_result.input_tokens,
                    tokens_out = exec_result.output_tokens,
                    "Belief extraction completed for source"
                );
            }
            Err(e) => {
                let err_msg = format!("{}", e);
                recorder
                    .update_phase(
                        exec_row.id,
                        "failed",
                        None,
                        Some(&err_msg),
                        0,
                        0,
                        0.0,
                        Some(&extractor_cfg.model_id),
                    )
                    .await;

                warn!(
                    source = %source.title,
                    error = %err_msg,
                    "Belief extraction failed for source, continuing"
                );
            }
        }
    }

    // 6. Build structured output summary
    let mut by_type: HashMap<String, usize> = HashMap::new();
    let mut by_confidence: HashMap<String, usize> = HashMap::new();
    for b in &all_beliefs {
        *by_type.entry(b.belief_type.clone()).or_default() += 1;
        *by_confidence.entry(b.confidence.clone()).or_default() += 1;
    }

    let summary = serde_json::json!({
        "beliefs_extracted": all_beliefs.len(),
        "sources_processed": total_sources,
        "beliefs_by_type": by_type,
        "beliefs_by_confidence": by_confidence,
    });

    // 7. Store results
    *total_input_tokens += step_in_tokens;
    *total_output_tokens += step_out_tokens;
    *total_cost_usd += step_cost;

    let output_key = resolve_output_key(step, &port_meta.step_outputs);
    let output = StepOutput {
        variable_name: output_key,
        raw_output: serde_json::to_string_pretty(&summary).unwrap_or_default(),
        structured_output: Some(summary.clone()),
    };

    if !output.variable_name.is_empty() {
        if let Some(ref structured) = output.structured_output {
            var_outputs.insert(output.variable_name.clone(), structured.clone());
        }
    }

    let envelope = StepExecutionEnvelope {
        status: ExecutionStatus::Success,
        data: Some(summary),
        metadata: ExecutionMetadata {
            execution_id: step.id,
            execution_time_ms: step_start.elapsed().as_millis() as u64,
            tokens_in: Some(step_in_tokens as i32),
            tokens_out: Some(step_out_tokens as i32),
            cost_usd: Some(step_cost as f64),
            model: Some(extractor_cfg.model_id.clone()),
            agent_id: None,
            iteration_index: None,
            iteration_label: None,
            routing_label: None,
            upstream_agent_id: None,
            upstream_routing_label: None,
            room_session_id: None,
            room_id: None,
            total_rounds: None,
        },
        error: None,
    };
    completed_envelopes.insert(step.id, envelope);
    completed.insert(step.id, output);

    // 8. Broadcast step completed
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::BeliefExtractionProgress {
            step_id: step.id,
            source_step_name: String::new(),
            sources_completed: total_sources,
            sources_total: total_sources,
            beliefs_extracted: all_beliefs.len(),
        },
    );

    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            output: None,
            input_tokens: Some(step_in_tokens as u64),
            output_tokens: Some(step_out_tokens as u64),
            duration_ms: Some(step_start.elapsed().as_millis() as u64),
        },
    );

    info!(
        step_id = %step.id,
        sources = total_sources,
        beliefs = all_beliefs.len(),
        tokens_in = step_in_tokens,
        tokens_out = step_out_tokens,
        duration_ms = step_start.elapsed().as_millis(),
        "Belief capture step execution completed"
    );

    Ok(())
}

// ── Helper types and functions ──────────────────────────────────────────────

/// Parsed output from the LLM extraction call.
#[derive(Debug, Deserialize)]
struct BeliefExtractionOutput {
    beliefs: Vec<ExtractedBelief>,
}

/// A single belief extracted by the LLM.
#[derive(Debug, Deserialize)]
struct ExtractedBelief {
    content: String,
    reasoning: String,
    belief_type: String,
    confidence: String,
    #[serde(default)]
    confidence_justification: Option<String>,
    #[serde(default)]
    semantic_tags: Vec<String>,
    #[serde(default)]
    emotional_tone: Option<String>,
    #[serde(default)]
    cross_source_tension: Option<String>,
}

/// Parse the LLM response into a list of extracted beliefs.
fn parse_extraction_output(content: &str) -> Vec<ExtractedBelief> {
    match parse_structured_output(content) {
        Some(json) => match serde_json::from_value::<BeliefExtractionOutput>(json) {
            Ok(output) => output.beliefs,
            Err(e) => {
                warn!("Failed to deserialize belief extraction output: {}", e);
                vec![]
            }
        },
        None => {
            warn!("No structured JSON found in belief extraction response");
            vec![]
        }
    }
}

/// Check if a belief's confidence meets the minimum threshold.
///
/// Confidence levels: "high" = 3, "medium" = 2, "low" = 1.
/// Returns true if the belief's confidence >= the threshold.
fn confidence_meets_threshold(confidence: &str, threshold: &str) -> bool {
    let level = |s: &str| match s {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    };
    level(confidence) >= level(threshold)
}
