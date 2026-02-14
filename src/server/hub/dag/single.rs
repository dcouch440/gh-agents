//! Single step execution through the ExecutionEngine.
//!
//! Contains `execute_single_step` (the default execution path for non-special
//! step types) and `run_step_via_engine` (the shared low-level engine call
//! used by single and for-each steps).

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::db::{AgentRow, StepOutputRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::execution::ContainerHandle;
use crate::server::hub::engine::filters::{
    AgentGuidanceFilter, DebateVerificationFilter, DocumenterPromptFilter, ExecutionFilter,
    FewShotFilter, FilterContext, PartialJsonRecoveryFilter, ReasoningTraceFilter,
    SchemaEnhancementFilter, SchemaValidationRetryFilter,
};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::dag_step::{compute_cost, DagStepConfig, DagStepStrategy};
use crate::server::hub::streaming::NullSink;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::StepExecutionEnvelope;

use super::container::{
    create_optional_container, destroy_optional_container, run_with_vpn_watchdog,
};
use super::{
    broadcast_workflow_event, build_routing_instruction_block, compose_prompt,
    gather_downstream_routing_context, resolve_output_key, resolve_step_port_inputs,
    step_display_name, wrap_in_envelope, PortMetadata, StepOutput, WorkflowExecutionContext,
};

/// Execute a single (non-for-each) step through the engine.
pub(super) async fn execute_single_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
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

    // Broadcast: step started
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: Some(agent.id),
            execution_id: None,
        },
    );

    // Resolve port inputs if this step has input ports defined
    let port_inputs = resolve_step_port_inputs(step, edges, port_meta, completed_envelopes);

    let prompt = compose_prompt(
        step,
        state.prompt_template_repo().as_deref(),
        state.doc_repo().as_deref(),
        state.workflow_repo().as_deref(),
        &**state.repo(),
        var_outputs,
        &ctx.prior_outputs,
        None,
        port_inputs.as_ref(),
    )
    .await;

    // Phase 6: Inject downstream routing context into the prompt
    let mut prompt = prompt;
    let local_step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();
    let downstream_contexts =
        gather_downstream_routing_context(step.id, edges, &local_step_map, port_meta, state).await;
    for routing_ctx in &downstream_contexts {
        prompt.push_str(&build_routing_instruction_block(routing_ctx));
    }

    // Create container if configured (with optional VPN sidecar)
    let managed_container = create_optional_container(
        ctx.container_config.as_ref(),
        ctx.wg_client.as_deref(),
        "step",
    )
    .await?;

    let result = run_with_vpn_watchdog(
        &managed_container,
        run_step_via_engine(
            engine,
            state,
            ctx,
            step,
            agent,
            &prompt,
            &port_meta.step_outputs,
            cancel,
            managed_container.as_ref().map(|mc| &mc.agent_handle),
        ),
    )
    .await;

    destroy_optional_container(&managed_container, ctx.wg_client.as_deref()).await;

    let (output, in_tok, out_tok, cost) = result?;

    *total_input_tokens += in_tok;
    *total_output_tokens += out_tok;
    *total_cost_usd += cost;

    // Store output in variable map (fixes var_outputs propagation bug)
    if !output.variable_name.is_empty() {
        if let Some(ref structured) = output.structured_output {
            var_outputs.insert(output.variable_name.clone(), structured.clone());
        }
    }

    // Store envelope for downstream port resolution
    let envelope = wrap_in_envelope(&output, agent, step.id, in_tok, out_tok, cost);
    completed_envelopes.insert(step.id, envelope);

    completed.insert(step.id, output);

    // Broadcast: step completed
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: Some(agent.id),
            output: None,
            input_tokens: Some(in_tok as u64),
            output_tokens: Some(out_tok as u64),
            duration_ms: Some(step_start.elapsed().as_millis() as u64),
        },
    );

    Ok(())
}

/// Run a single step execution through the ExecutionEngine.
///
/// Creates the agent_execution record, builds a DagStepStrategy, and
/// calls `engine.execute()`. Returns (StepOutput, input_tokens, output_tokens, cost).
pub(crate) async fn run_step_via_engine(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    prompt: &str,
    step_outputs: &HashMap<Uuid, Vec<StepOutputRow>>,
    cancel: Option<&CancellationToken>,
    container_handle: Option<&ContainerHandle>,
) -> Result<(StepOutput, i64, i64, f32), HubError> {
    let ae_repo = state
        .agent_execution_repo()
        .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;

    // Load agent tools
    let agent_tool_rows = state
        .repo()
        .get_agent_tools(agent.id)
        .await
        .unwrap_or_default();
    let tools: Vec<crate::llm::Tool> = agent_tool_rows
        .iter()
        .filter_map(|row| crate::tools::registry::get_tool_definition(&row.name))
        .collect();
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();

    // Build system prompt: agent base + step suffix + schema enforcement
    let mut system_prompt = agent.system_prompt.clone();
    if let Some(ref suffix) = step.system_prompt_suffix {
        if !suffix.trim().is_empty() {
            system_prompt.push_str("\n\n");
            system_prompt.push_str(suffix);
        }
    }
    let mut output_schema_value: Option<JsonValue> = None;
    if let Some(schema_id) = step.output_schema_id {
        let os_repo = &state.repos().output_schemas;
        if let Ok(Some(schema)) = os_repo.get_output_schema(schema_id).await {
            system_prompt.push_str(&format!(
                "\n\n<schema>\nYour response is parsed directly by a JSON parser. Respond with a valid JSON object matching this schema:\n```json\n{}\n```\n</schema>",
                serde_json::to_string_pretty(&schema.schema).unwrap_or_default()
            ));
            output_schema_value = Some(schema.schema.clone());
        }
    }

    // Create agent_execution row
    let ae_row = ae_repo
        .create_agent_execution(
            agent.id,
            Some(step.id),
            false,
            None,
            &system_prompt,
            prompt,
            None, // selected_mode_id (unused)
            None,
            None,
            Some(ctx.stage_execution_id),
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to create agent execution: {}", e))?;

    // Record initial messages
    let _ = ae_repo
        .create_execution_message(ae_row.id, "system", &system_prompt, None, 0, 0)
        .await;
    let _ = ae_repo
        .create_execution_message(ae_row.id, "user", prompt, None, 0, 0)
        .await;

    // Build strategy
    let config = DagStepConfig {
        agent: agent.clone(),
        step: step.clone(),
        system_prompt,
        user_prompt: prompt.to_string(),
        tools,
        tool_names,
        temperature: agent.model_temperature,
        execution_context: ctx.execution_context.clone(),
        container_handle: container_handle.cloned(),
        run_id: ctx.run_id,
        user_id: ctx.user_id,
        agent_execution_id: ae_row.id,
    };
    let strategy = DagStepStrategy::new(config, state.clone());

    // Build recorder (strategy handles its own recording in on_complete)
    let ae_repo = state.agent_execution_repo();
    let tl_repo = state.token_ledger_repo();
    let recorder = ExecutionRecorder::new(
        state.repo().as_ref(),
        ae_repo.as_deref(),
        tl_repo.as_deref(),
    );

    let sink = NullSink;

    // Build filter pipeline
    let mut filter_ctx = FilterContext::new(&agent.model_id, agent.id).with_step_id(step.id);
    filter_ctx.metadata.insert(
        "agent_execution_id".into(),
        serde_json::to_value(ae_row.id).unwrap(),
    );
    filter_ctx
        .metadata
        .insert("user_id".into(), serde_json::to_value(ctx.user_id).unwrap());
    filter_ctx.metadata.insert(
        "workflow_execution_id".into(),
        serde_json::to_value(ctx.stage_execution_id).unwrap(),
    );

    let mut filters: Vec<Arc<dyn ExecutionFilter>> =
        vec![Arc::new(AgentGuidanceFilter::new(state.repo().clone()))];

    if let Some(ae_repo) = state.agent_execution_repo() {
        filters.push(Arc::new(FewShotFilter::new(ae_repo)));
    }

    // Documenter: inject document definitions into system prompt
    if step.execution_mode == "documenter" {
        filters.push(Arc::new(DocumenterPromptFilter::new(
            state.repos().workflows.clone(),
        )));
    }

    if let Some(schema_val) = output_schema_value {
        filter_ctx = filter_ctx.with_schema(schema_val);
        filters.push(Arc::new(SchemaEnhancementFilter::new()));
        filters.push(Arc::new(SchemaValidationRetryFilter::new()));
        filters.push(Arc::new(PartialJsonRecoveryFilter::new()));
    }

    if step.reasoning_trace && filter_ctx.has_output_schema {
        filters.push(Arc::new(ReasoningTraceFilter::new()));
    }

    // Multi-agent debate/verification filter
    let verification_ids: Vec<Uuid> = step
        .verification_agent_ids
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    if !verification_ids.is_empty() {
        if let Some(provider) = state.provider() {
            filters.push(Arc::new(DebateVerificationFilter::new(
                provider.clone(),
                state.repo().clone(),
                verification_ids,
                state.agent_execution_repo(),
                state.token_ledger_repo(),
            )));
        }
    }

    let filtered_engine = engine
        .clone_with_provider()
        .with_filters(filters)
        .with_filter_context(filter_ctx);

    // Execute
    let result = filtered_engine
        .execute(&strategy, prompt, &sink, &recorder, cancel)
        .await?;

    let cost = compute_cost(
        &agent.model_id,
        result.input_tokens as i64,
        result.output_tokens as i64,
    );

    let variable_name = resolve_output_key(step, step_outputs);
    let structured =
        crate::server::hub::strategies::dag_step::DagStepStrategy::parse_output(&result.content);

    let output = StepOutput {
        variable_name,
        structured_output: structured,
        raw_output: result.content,
    };

    Ok((
        output,
        result.input_tokens as i64,
        result.output_tokens as i64,
        cost,
    ))
}
