//! Single step execution through the ExecutionEngine.
//!
//! Contains `execute_single_step` (the default execution path for non-special
//! step types) and `run_step_via_engine` (the shared low-level engine call
//! used by single and for-each steps).

mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::db::{AgentRow, WorkflowStepRow};
use crate::execution::ContainerHandle;
use crate::server::hub::engine::filters::{
    AgentGuidanceFilter, DebateVerificationFilter, ExecutionFilter, FewShotFilter, FilterContext,
    PartialJsonRecoveryFilter, ReasoningTraceFilter, SchemaEnhancementFilter,
    SchemaValidationRetryFilter,
};
use crate::server::hub::error::HubError;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::dag_step::{compute_cost, DagStepConfig, DagStepStrategy};
use crate::server::hub::streaming::NullSink;
use crate::server::ws::events::WorkflowEventKind;

use super::container::{
    create_optional_container, destroy_optional_container, run_with_vpn_watchdog,
};
use super::dag_state::DagExecutionState;
use super::{
    broadcast_workflow_event, build_routing_instruction_block, compose_prompt,
    gather_downstream_routing_context, resolve_output_key, resolve_step_port_inputs,
    step_display_name, wrap_in_envelope, DagContext, PromptRepos, StepOutput,
};

/// Execute a single (non-for-each) step through the engine.
pub(super) async fn execute_single_step(
    dag: &DagContext<'_>,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    dag_state: &mut DagExecutionState,
) -> Result<(), HubError> {
    let step_start = std::time::Instant::now();

    // Broadcast: step started
    broadcast_workflow_event(
        dag.state,
        dag.ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: Some(agent.id),
            execution_id: None,
        },
    );

    // Resolve port inputs if this step has input ports defined
    let port_inputs = resolve_step_port_inputs(
        step,
        dag.edges,
        dag.port_meta,
        &dag_state.completed_envelopes,
    );

    let pt_repo = dag.state.prompt_template_repo();
    let doc_repo = dag.state.doc_repo();
    let wf_repo = dag.state.workflow_repo();
    let repos = PromptRepos {
        prompt_template_repo: pt_repo.as_deref(),
        doc_repo: doc_repo.as_deref(),
        workflow_repo: wf_repo.as_deref(),
        server_repo: &**dag.state.repo(),
    };
    let prompt = compose_prompt(
        step,
        &repos,
        &dag_state.var_outputs,
        &dag.ctx.prior_outputs,
        None,
        port_inputs.as_ref(),
    )
    .await;

    // Snapshot composed user prompt for run history
    let _ = super::versioning::snapshot_content(
        &*dag.state.repos().content_versions,
        dag.ctx.run_id,
        step.id,
        step.id,
        super::versioning::content_types::PROMPT,
        "input",
        &prompt,
    )
    .await;

    // Phase 6: Inject downstream routing context into the prompt
    let mut prompt = prompt;
    let local_step_map: HashMap<Uuid, &WorkflowStepRow> =
        dag.steps.iter().map(|s| (s.id, s)).collect();
    let downstream_contexts = gather_downstream_routing_context(
        step.id,
        dag.edges,
        &local_step_map,
        dag.port_meta,
        dag.state,
    )
    .await;
    for routing_ctx in &downstream_contexts {
        prompt.push_str(&build_routing_instruction_block(routing_ctx));
    }

    // Create container if configured (with optional VPN sidecar)
    let managed_container = create_optional_container(
        dag.ctx.container_config.as_ref(),
        dag.ctx.wg_client.as_deref(),
        "step",
    )
    .await?;

    let result = run_with_vpn_watchdog(
        &managed_container,
        run_step_via_engine(
            dag,
            step,
            agent,
            &prompt,
            managed_container.as_ref().map(|mc| &mc.agent_handle),
        ),
    )
    .await;

    destroy_optional_container(&managed_container, dag.ctx.wg_client.as_deref()).await;

    let (output, in_tok, out_tok, cost) = result?;

    dag_state.accumulate_tokens(in_tok, out_tok, cost);

    // Store envelope for downstream port resolution
    let envelope = wrap_in_envelope(&output, agent, step.id, in_tok, out_tok, cost);

    // Snapshot envelope for run history
    let envelope_json = serde_json::to_string(&envelope).unwrap_or_default();
    dag_state.record_step_output(step.id, output, envelope);
    let _ = super::versioning::snapshot_content(
        &*dag.state.repos().content_versions,
        dag.ctx.run_id,
        step.id,
        step.id,
        super::versioning::content_types::ENVELOPE,
        "output",
        &envelope_json,
    )
    .await;

    // Broadcast: step completed
    broadcast_workflow_event(
        dag.state,
        dag.ctx,
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
    dag: &DagContext<'_>,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    prompt: &str,
    container_handle: Option<&ContainerHandle>,
) -> Result<(StepOutput, i64, i64, f32), HubError> {
    let ae_repo = dag
        .state
        .agent_execution_repo()
        .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;

    // Load agent tools (from snapshot if template-based, else from live DB)
    let agent_tool_rows = if let Some(snap) = &dag.ctx.snapshot {
        snap.agent_tools.get(&agent.id).cloned().unwrap_or_default()
    } else {
        dag.state
            .repo()
            .get_agent_tools(agent.id)
            .await
            .unwrap_or_default()
    };
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
        let os_repo = &dag.state.repos().output_schemas;
        if let Ok(Some(schema)) = os_repo.get_output_schema(schema_id).await {
            system_prompt.push_str(&format!(
                "\n\n<schema>\nYour response is parsed directly by a JSON parser. Respond with a valid JSON object matching this schema:\n```json\n{}\n```\n</schema>",
                serde_json::to_string_pretty(&schema.schema).unwrap_or_default()
            ));
            output_schema_value = Some(schema.schema.clone());
        }
    }

    // Snapshot system prompt for run history
    let _ = super::versioning::snapshot_content(
        &*dag.state.repos().content_versions,
        dag.ctx.run_id,
        step.id,
        step.id,
        super::versioning::content_types::SYSTEM_PROMPT,
        "input",
        &system_prompt,
    )
    .await;

    // Create agent_execution row
    let ae_row = ae_repo
        .create_agent_execution(
            agent.id,
            Some(step.id),
            false,
            None,
            &system_prompt,
            prompt,
            None,
            None,
            Some(dag.ctx.stage_execution_id),
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
        execution_context: dag.ctx.execution_context.clone(),
        container_handle: container_handle.cloned(),
        run_id: dag.ctx.run_id,
        user_id: dag.ctx.user_id,
        agent_execution_id: ae_row.id,
    };
    let strategy = DagStepStrategy::new(config, dag.state.clone());

    // Build recorder (strategy handles its own recording in on_complete)
    let ae_repo = dag.state.agent_execution_repo();
    let tl_repo = dag.state.token_ledger_repo();
    let recorder = ExecutionRecorder::new(
        dag.state.repo().as_ref(),
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
    filter_ctx.metadata.insert(
        "user_id".into(),
        serde_json::to_value(dag.ctx.user_id).unwrap(),
    );
    filter_ctx.metadata.insert(
        "workflow_execution_id".into(),
        serde_json::to_value(dag.ctx.stage_execution_id).unwrap(),
    );

    let mut filters: Vec<Arc<dyn ExecutionFilter>> =
        vec![Arc::new(AgentGuidanceFilter::new(dag.state.repo().clone()))];

    if let Some(ae_repo) = dag.state.agent_execution_repo() {
        filters.push(Arc::new(FewShotFilter::new(ae_repo)));
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
        if let Some(provider) = dag.state.provider() {
            filters.push(Arc::new(DebateVerificationFilter::new(
                provider.clone(),
                dag.state.repo().clone(),
                verification_ids,
                dag.state.agent_execution_repo(),
                dag.state.token_ledger_repo(),
            )));
        }
    }

    let filtered_engine = dag
        .engine
        .clone_with_provider()
        .with_filters(filters)
        .with_filter_context(filter_ctx);

    // Execute
    let result = filtered_engine
        .execute(&strategy, prompt, &sink, &recorder, dag.cancel)
        .await?;

    let cost = compute_cost(
        &agent.model_id,
        result.input_tokens as i64,
        result.output_tokens as i64,
    );

    let variable_name = resolve_output_key(step, &dag.port_meta.step_outputs);
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
