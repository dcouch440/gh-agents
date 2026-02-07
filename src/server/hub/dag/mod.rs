//! DAG orchestration — topological sort, variable resolution, for-each fan-out,
//! port-based data flow, and workflow execution using the unified ExecutionEngine.
//!
//! Pure utility functions live in the `utils` submodule and are re-exported here.
//! Execution functions use the hub's `ExecutionEngine` for step execution.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::traits::WorkflowCollectionRepo;
use crate::db::{
    AgentRow, StepInputRow, StepOutputRow, StepRoutingRuleRow, WorkflowStepEdgeRow, WorkflowStepRow,
};
use crate::execution::{
    ContainerConfig, ContainerHandle, ContainerManager, VpnSidecarHandle, VpnSidecarManager,
    WgEasyClient,
};
use crate::server::state::AppState;
use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};
use crate::types::{
    DownstreamRoutingContext, ExecutionMetadata, ExecutionStatus, RouteDescription,
    StepExecutionEnvelope,
};

use super::construct_agent_defaults;
use super::engine::filters::{
    AgentGuidanceFilter, DebateVerificationFilter, ExecutionFilter, FewShotFilter, FilterContext,
    PartialJsonRecoveryFilter, ReasoningTraceFilter, SchemaEnhancementFilter,
    SchemaValidationRetryFilter,
};
use super::engine::ExecutionEngine;
use super::error::HubError;
use super::recorder::ExecutionRecorder;
use super::strategies::cavernous::{CavernousStepConfig, CavernousStepStrategy};
use super::strategies::dag_step::{compute_cost, DagStepConfig, DagStepStrategy};
use super::streaming::NullSink;

use crate::types::{DocumentSummary, RoutingConfigDocument, Subtask};

use crate::server::executors::room::{
    build_dag_room_prompt, execute_room_turn, RoomMemberWithAgent,
};

/// Emit a workflow lifecycle event via WebSocket broadcast.
fn broadcast_workflow_event(
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    workflow_id: Uuid,
    kind: WorkflowEventKind,
) {
    state.broadcast_workflow(WorkflowEvent {
        run_id: ctx.run_id,
        workflow_id,
        user_id: Some(ctx.user_id),
        kind,
    });
}

pub mod utils;
pub use utils::{
    build_routing_instruction_block, compose_prompt, extract_for_each_label, find_entry_steps,
    get_child_steps, get_parent_steps, resolve_dot_path, resolve_for_each_array,
    resolve_port_inputs, resolve_variables, topological_sort, ContainerExecutionConfig, DagPaused,
    PortResolutionError, StepOutput, WorkflowExecutionContext, WorkflowExecutionResult,
};

/// Determine the output key for a step: prefer the first output port name,
/// fall back to the legacy `output_variable_name` field.
fn resolve_output_key(
    step: &WorkflowStepRow,
    step_outputs: &HashMap<Uuid, Vec<StepOutputRow>>,
) -> String {
    if let Some(ports) = step_outputs.get(&step.id) {
        if let Some(first) = ports.first() {
            if !first.port_name.is_empty() {
                return first.port_name.clone();
            }
        }
    }
    step.output_variable_name.clone().unwrap_or_default()
}

/// Wrap a step output into a StepExecutionEnvelope for port-based data flow.
fn wrap_in_envelope(
    output: &StepOutput,
    agent: &AgentRow,
    execution_id: Uuid,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
) -> StepExecutionEnvelope {
    StepExecutionEnvelope {
        status: if output.structured_output.is_some() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Error
        },
        data: output.structured_output.clone(),
        metadata: ExecutionMetadata {
            execution_id,
            execution_time_ms: 0,
            tokens_in: Some(input_tokens as i32),
            tokens_out: Some(output_tokens as i32),
            cost_usd: Some(cost_usd as f64),
            model: Some(agent.model_id.clone()),
            agent_id: Some(agent.id),
            iteration_index: None,
            iteration_label: None,
            routing_label: None,
            selected_routing_document_id: None,
            upstream_agent_id: None,
            upstream_routing_label: None,
            room_session_id: None,
            room_id: None,
            total_rounds: None,
        },
        error: None,
    }
}

/// Pre-fetched port metadata for all steps in a workflow.
struct PortMetadata {
    step_inputs: HashMap<Uuid, Vec<StepInputRow>>,
    step_outputs: HashMap<Uuid, Vec<StepOutputRow>>,
    routing_rules: HashMap<Uuid, Vec<StepRoutingRuleRow>>,
}

/// Pre-fetch port metadata (inputs, outputs, routing rules) for all steps.
async fn prefetch_port_metadata(state: &AppState, steps: &[WorkflowStepRow]) -> PortMetadata {
    let mut step_inputs: HashMap<Uuid, Vec<StepInputRow>> = HashMap::new();
    let mut step_outputs: HashMap<Uuid, Vec<StepOutputRow>> = HashMap::new();
    let mut routing_rules: HashMap<Uuid, Vec<StepRoutingRuleRow>> = HashMap::new();

    if let Some(ref wf_repo) = state.workflow_repo() {
        for step in steps {
            if let Ok(inputs) = wf_repo.get_step_inputs(step.id).await {
                if !inputs.is_empty() {
                    step_inputs.insert(step.id, inputs);
                }
            }
            if let Ok(outputs) = wf_repo.get_step_outputs(step.id).await {
                if !outputs.is_empty() {
                    step_outputs.insert(step.id, outputs);
                }
            }
            if step.routing_mode.as_deref() == Some("label") {
                if let Ok(rules) = wf_repo.get_step_routing_rules(step.id).await {
                    if !rules.is_empty() {
                        routing_rules.insert(step.id, rules);
                    }
                }
            }
        }
    }

    PortMetadata {
        step_inputs,
        step_outputs,
        routing_rules,
    }
}

// ============================================================================
// Phase 6B: Chained For-Each Pipeline Detection
// ============================================================================

/// A contiguous chain of for-each steps connected by single edges.
/// Items flow through the chain without barriers between stages.
#[derive(Debug, Clone)]
struct ForEachChain {
    /// Ordered step IDs — first is the entry, last feeds the barrier.
    step_ids: Vec<Uuid>,
}

/// Detect chains of consecutive for-each steps.
///
/// A chain is a maximal sequence `[S1, S2, ..., Sn]` (n >= 2) where:
/// - Every step has `execution_mode == "for_each"`
/// - Each `S_{i+1}` has exactly one parent that is `S_i`
/// - Each `S_i` has exactly one for-each child that is `S_{i+1}`
///
/// Fan-out (multiple children) and fan-in (multiple parents) break chains.
fn detect_for_each_chains(
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> Vec<ForEachChain> {
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

    // Build adjacency: children and parents per step
    let mut children: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    let mut parents: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for edge in edges {
        children
            .entry(edge.from_step_id)
            .or_default()
            .push(edge.to_step_id);
        parents
            .entry(edge.to_step_id)
            .or_default()
            .push(edge.from_step_id);
    }

    let mut claimed: HashSet<Uuid> = HashSet::new();
    let mut chains = Vec::new();

    for step in steps {
        if claimed.contains(&step.id) {
            continue;
        }
        if step.execution_mode != "for_each" {
            continue;
        }

        // Start building a chain from this step
        let mut chain_ids = vec![step.id];
        let mut current = step.id;

        loop {
            let step_children = children.get(&current).cloned().unwrap_or_default();

            // Find for-each children
            let fe_children: Vec<Uuid> = step_children
                .iter()
                .filter(|cid| {
                    step_map
                        .get(cid)
                        .is_some_and(|s| s.execution_mode == "for_each")
                })
                .copied()
                .collect();

            // Must have exactly one for-each child to continue chain
            if fe_children.len() != 1 {
                break;
            }

            let next = fe_children[0];

            // The child must have exactly one parent (current step)
            let child_parents = parents.get(&next).cloned().unwrap_or_default();
            if child_parents.len() != 1 || child_parents[0] != current {
                break;
            }

            // Don't re-claim
            if claimed.contains(&next) {
                break;
            }

            chain_ids.push(next);
            current = next;
        }

        // Only record chains of length >= 2
        if chain_ids.len() >= 2 {
            for id in &chain_ids {
                claimed.insert(*id);
            }
            chains.push(ForEachChain {
                step_ids: chain_ids,
            });
        }
    }

    chains
}

/// For a given step, find downstream label-routing steps and build
/// routing context for prompt injection.
///
/// Uses edges (in memory), the step map, and pre-fetched routing rules.
/// Loads agent names and tools from the database on demand.
async fn gather_downstream_routing_context(
    step_id: Uuid,
    edges: &[WorkflowStepEdgeRow],
    step_map: &HashMap<Uuid, &WorkflowStepRow>,
    port_meta: &PortMetadata,
    state: &AppState,
) -> Vec<DownstreamRoutingContext> {
    let mut contexts = Vec::new();

    let child_step_ids = get_child_steps(step_id, edges);

    for child_id in child_step_ids {
        let Some(child_step) = step_map.get(&child_id) else {
            continue;
        };

        if child_step.routing_mode.as_deref() != Some("label") {
            continue;
        }

        let Some(routing_field) = child_step.routing_field.as_ref() else {
            continue;
        };

        let Some(rules) = port_meta.routing_rules.get(&child_id) else {
            continue;
        };

        if rules.is_empty() {
            continue;
        }

        let mut routes = Vec::new();
        for rule in rules {
            let agent_name = match state.repo().get_persisted_agent(rule.agent_id).await {
                Ok(Some(agent)) => agent.name,
                _ => format!("Agent {}", rule.agent_id),
            };

            let agent_tools = match state.repo().get_agent_tools(rule.agent_id).await {
                Ok(tools) => tools.into_iter().map(|t| t.name).collect(),
                Err(_) => vec![],
            };

            routes.push(RouteDescription {
                label_value: rule.label_value.clone(),
                description: rule.description.clone(),
                agent_name,
                agent_tools,
            });
        }

        contexts.push(DownstreamRoutingContext {
            downstream_step_id: child_id,
            routing_field: routing_field.clone(),
            routes,
        });
    }

    contexts
}

/// Execute a complete workflow DAG using the unified ExecutionEngine.
///
/// Executes a DAG via topo sort, variable resolution, for-each fan-out,
/// and interactive review. Step execution goes through
/// `ExecutionEngine::execute()` with `DagStepStrategy`.
///
/// Supports port-based data flow: if steps define input/output ports and edges
/// connect them, data flows through envelopes with structured extraction.
pub async fn execute_workflow_via_engine(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    cancel: Option<&CancellationToken>,
) -> Result<WorkflowExecutionResult, HubError> {
    let sorted = topological_sort(steps, edges).map_err(|_| HubError::DagCycle)?;
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();
    let workflow_id = steps.first().map(|s| s.workflow_id).unwrap_or(Uuid::nil());
    let start_time = std::time::Instant::now();

    let mut completed: HashMap<Uuid, StepOutput> = HashMap::new();
    let mut completed_envelopes: HashMap<Uuid, StepExecutionEnvelope> = HashMap::new();
    let mut var_outputs: HashMap<String, JsonValue> = HashMap::new();
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f32 = 0.0;

    // Pre-fetch port metadata for all steps
    let port_meta = prefetch_port_metadata(state, steps).await;

    // Broadcast: workflow started
    broadcast_workflow_event(
        state,
        ctx,
        workflow_id,
        WorkflowEventKind::Started {
            total_steps: sorted.len(),
        },
    );

    // Phase 6B: Detect chained for-each pipelines
    let chains = detect_for_each_chains(steps, edges);
    let chain_member_set: HashSet<Uuid> = chains
        .iter()
        .flat_map(|c| c.step_ids.iter().copied())
        .collect();

    if !chains.is_empty() {
        info!(
            chain_count = chains.len(),
            "Detected chained for-each pipelines"
        );
    }

    for step_id in &sorted {
        // Skip steps already executed as part of a chain
        if completed.contains_key(step_id) {
            continue;
        }

        let step = match step_map.get(step_id) {
            Some(s) => *s,
            None => continue,
        };

        // Check cancellation before each step
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        // Check all parents are completed
        let parents = get_parent_steps(*step_id, edges);
        let all_parents_done = parents.iter().all(|pid| completed.contains_key(pid));
        if !all_parents_done {
            warn!("Step {} has uncompleted parents, skipping", step_id);
            continue;
        }

        // Phase 6B: If this is the head of a for-each chain, execute the whole chain
        if let Some(chain) = chains.iter().find(|c| c.step_ids[0] == *step_id) {
            execute_for_each_chain(
                engine,
                state,
                ctx,
                chain,
                &step_map,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
            continue;
        }

        // Skip non-head chain members (already executed by chain head)
        if chain_member_set.contains(step_id) {
            continue;
        }

        // Load agent
        let agent = state
            .repo()
            .get_persisted_agent(step.agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: *step_id,
                agent_id: step.agent_id,
            })?;

        // Resolve provider: use registry if agent targets non-default provider
        let step_engine = if agent.model_provider == "anthropic" || agent.model_provider.is_empty()
        {
            None // Use default engine
        } else {
            // Check runtime toggle for non-default providers
            if agent.model_provider == "ollama" && !state.is_ollama_enabled().await {
                return Err(HubError::ProviderUnavailable {
                    provider: agent.model_provider.clone(),
                    step_id: *step_id,
                    agent_name: agent.name.clone(),
                });
            }
            let provider = state.provider_for(&agent.model_provider).ok_or_else(|| {
                HubError::ProviderUnavailable {
                    provider: agent.model_provider.clone(),
                    step_id: *step_id,
                    agent_name: agent.name.clone(),
                }
            })?;
            Some(ExecutionEngine::new(provider))
        };
        let effective_engine = step_engine.as_ref().unwrap_or(engine);

        let step_result = if step.execution_mode == "room" {
            execute_room_step(
                effective_engine,
                state,
                ctx,
                step,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await
        } else if step.execution_mode == "cavernous" {
            execute_cavernous_step(
                effective_engine,
                state,
                ctx,
                step,
                &agent,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await
        } else if step.execution_mode == "for_each" {
            execute_for_each_step(
                effective_engine,
                state,
                ctx,
                step,
                &agent,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await
        } else {
            execute_single_step(
                effective_engine,
                state,
                ctx,
                step,
                &agent,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await
        };

        if let Err(ref e) = step_result {
            if !matches!(e, HubError::AwaitingUser { .. }) {
                broadcast_workflow_event(
                    state,
                    ctx,
                    workflow_id,
                    WorkflowEventKind::StepFailed {
                        step_id: step.id,
                        step_name: step
                            .output_variable_name
                            .clone()
                            .unwrap_or_else(|| step.id.to_string()),
                        error: format!("{}", e),
                    },
                );
                broadcast_workflow_event(
                    state,
                    ctx,
                    workflow_id,
                    WorkflowEventKind::Failed {
                        error: format!(
                            "Step '{}' failed: {}",
                            step.output_variable_name.as_deref().unwrap_or("unknown"),
                            e
                        ),
                    },
                );
            }
        }
        step_result?;
    }

    // Broadcast: workflow completed
    broadcast_workflow_event(
        state,
        ctx,
        workflow_id,
        WorkflowEventKind::Completed {
            duration_ms: Some(start_time.elapsed().as_millis() as u64),
        },
    );

    let final_outputs: HashMap<String, StepOutput> = completed
        .into_iter()
        .map(|(id, out)| (id.to_string(), out))
        .collect();

    Ok(WorkflowExecutionResult {
        outputs: final_outputs,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
    })
}

/// A managed container that may include a VPN sidecar.
struct ManagedContainer {
    /// The agent container handle (for tool execution).
    agent_handle: ContainerHandle,
    /// Optional VPN sidecar (must be cleaned up separately).
    vpn_sidecar: Option<VpnSidecarHandle>,
}

/// Create a container if config is present, with optional VPN sidecar.
///
/// Returns `Ok(None)` if config is `None` (local execution).
/// Returns `Ok(Some(managed))` on success, `Err` on failure.
async fn create_optional_container(
    config: Option<&utils::ContainerExecutionConfig>,
    wg_client: Option<&WgEasyClient>,
    label: &str,
) -> Result<Option<ManagedContainer>, HubError> {
    let Some(cc) = config else {
        return Ok(None);
    };

    // If VPN is enabled, create a sidecar first
    let vpn_sidecar = if cc.vpn_enabled {
        let wg = wg_client.ok_or_else(|| {
            HubError::Internal(anyhow!("VPN enabled but wg-easy client not configured"))
        })?;

        use crate::execution::vpn::retry::vpn_with_retry;

        let peer_name = format!("{}-{}", label, Uuid::new_v4());
        let peer = vpn_with_retry(|| wg.create_peer(&peer_name))
            .await
            .map_err(|e| HubError::Internal(anyhow!("VPN peer creation failed: {}", e)))?;

        let peer_id_for_config = peer.id.clone();
        let peer_config = vpn_with_retry(|| wg.get_peer_config(&peer_id_for_config))
            .await
            .map_err(|e| HubError::Internal(anyhow!("VPN peer config failed: {}", e)))?;

        crate::execution::vpn::validate_wg_config(&peer_config)
            .map_err(|e| HubError::Internal(anyhow!("VPN config validation failed: {}", e)))?;

        let sidecar = match VpnSidecarManager::create_sidecar(&peer_config, &peer.id).await {
            Ok(s) => s,
            Err(e) => {
                warn!(peer_id = %peer.id, error = %e, "VPN sidecar failed, cleaning up peer");
                if let Some(wg) = wg_client {
                    if let Err(del_err) = wg.delete_peer(&peer.id).await {
                        warn!(
                            peer_id = %peer.id,
                            error = %del_err,
                            "Failed to clean up orphaned VPN peer"
                        );
                    }
                }
                return Err(HubError::Internal(anyhow!(
                    "VPN sidecar creation failed: {}",
                    e
                )));
            }
        };

        info!(
            container = %sidecar.container_name,
            peer_id = %peer.id,
            label,
            "VPN sidecar ready"
        );
        Some(sidecar)
    } else {
        None
    };

    // Build container config, optionally sharing VPN sidecar's network
    let container_config = ContainerConfig {
        clone_url: cc.clone_url.clone(),
        branch: cc.branch.clone(),
        github_token: cc.github_token.clone(),
        image: cc
            .image
            .clone()
            .unwrap_or_else(|| crate::constants::CONTAINER_DEFAULT_IMAGE.to_string()),
        memory_limit: cc
            .memory_limit
            .clone()
            .unwrap_or_else(|| crate::constants::CONTAINER_DEFAULT_MEMORY.to_string()),
        cpu_limit: cc
            .cpu_limit
            .clone()
            .unwrap_or_else(|| crate::constants::CONTAINER_DEFAULT_CPUS.to_string()),
        network_mode: vpn_sidecar
            .as_ref()
            .map(|s| format!("container:{}", s.container_id)),
        ..ContainerConfig::default()
    };

    use crate::execution::container::retry::container_with_retry;

    let container_mgr = ContainerManager::real();
    match container_with_retry(|| container_mgr.create_container(&container_config)).await {
        Ok(handle) => {
            info!(container = %handle.container_name(), label, "Created container");
            Ok(Some(ManagedContainer {
                agent_handle: handle,
                vpn_sidecar,
            }))
        }
        Err(e) => {
            // Clean up VPN sidecar if agent container creation fails
            if let Some(ref sidecar) = vpn_sidecar {
                VpnSidecarManager::destroy_sidecar_quiet(sidecar).await;
            }
            error!(label, error = %e, "Failed to create container");
            Err(HubError::Internal(anyhow!(
                "Container creation failed: {}",
                e
            )))
        }
    }
}

/// Destroy a managed container (agent + optional VPN sidecar + peer cleanup).
async fn destroy_optional_container(
    managed: &Option<ManagedContainer>,
    wg_client: Option<&WgEasyClient>,
) {
    let Some(ref mc) = managed else { return };

    // 1. Destroy agent container first
    ContainerManager::destroy_container_quiet(&mc.agent_handle).await;

    // 2. Destroy VPN sidecar if present
    if let Some(ref sidecar) = mc.vpn_sidecar {
        VpnSidecarManager::destroy_sidecar_quiet(sidecar).await;

        // 3. Delete wg-easy peer (with retry)
        if let Some(wg) = wg_client {
            use crate::execution::vpn::retry::vpn_with_retry;
            let peer_id = sidecar.peer_id.clone();
            if let Err(e) = vpn_with_retry(|| wg.delete_peer(&peer_id)).await {
                warn!(peer_id = %sidecar.peer_id, error = %e, "Failed to delete VPN peer");
            }
        }
    }
}

/// Run an execution future with a VPN tunnel watchdog.
///
/// If the managed container has a VPN sidecar, races the execution against
/// [`monitor_vpn_tunnel`]. If the tunnel drops, execution is cancelled and
/// an error is returned. If no VPN sidecar is present, execution runs directly.
async fn run_with_vpn_watchdog<F, T>(
    managed: &Option<ManagedContainer>,
    execution: F,
) -> Result<T, HubError>
where
    F: std::future::Future<Output = Result<T, HubError>>,
{
    use crate::execution::vpn_sidecar::watchdog::monitor_vpn_tunnel;

    let sidecar_id = managed
        .as_ref()
        .and_then(|mc| mc.vpn_sidecar.as_ref())
        .map(|s| s.container_id.as_str());

    match sidecar_id {
        Some(id) => {
            tokio::select! {
                result = execution => result,
                vpn_err = monitor_vpn_tunnel(
                    id,
                    crate::constants::VPN_WATCHDOG_INTERVAL_SECS,
                    crate::constants::VPN_WATCHDOG_MAX_FAILURES,
                ) => {
                    Err(HubError::Internal(anyhow!(
                        "VPN tunnel dropped during execution: {}", vpn_err
                    )))
                }
            }
        }
        None => execution.await,
    }
}

/// Execute a single (non-for-each) step through the engine.
async fn execute_single_step(
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
            step_name: step
                .output_variable_name
                .clone()
                .unwrap_or_else(|| step.id.to_string()),
            agent_id: Some(agent.id),
            execution_id: None,
        },
    );

    // Resolve port inputs if this step has input ports defined
    let port_inputs = if let Some(inputs) = port_meta.step_inputs.get(&step.id) {
        match resolve_port_inputs(
            step.id,
            edges,
            inputs,
            &port_meta.step_outputs,
            completed_envelopes,
        ) {
            Ok(resolved) => {
                debug!(step_id = %step.id, ports = resolved.len(), "Resolved port inputs");
                Some(resolved)
            }
            Err(e) => {
                warn!("Port resolution failed for step {}: {}", step.id, e);
                None
            }
        }
    } else {
        None
    };

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
            step_name: step
                .output_variable_name
                .clone()
                .unwrap_or_else(|| step.id.to_string()),
            agent_id: Some(agent.id),
            output: None,
            input_tokens: Some(in_tok as u64),
            output_tokens: Some(out_tok as u64),
            duration_ms: Some(step_start.elapsed().as_millis() as u64),
        },
    );

    Ok(())
}

// ============================================================================
// Room Step Execution
// ============================================================================

/// Execute a room step within the DAG.
///
/// Two modes:
/// - **Auto-run**: All rounds execute automatically, agents discuss via `execute_room_turn()`.
/// - **Interactive** (`agent_execution_mode = "interactive"`): Agents run an initial round,
///   then the DAG pauses for user participation. The user chats via the normal room API
///   and closes the session to resume the DAG.
///
/// Produces a composite envelope with per-agent outputs: `{"agent:<uuid>": output, ...}`.
#[allow(clippy::too_many_arguments)]
async fn execute_room_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    _steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    var_outputs: &mut HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    completed_envelopes: &mut HashMap<Uuid, StepExecutionEnvelope>,
    port_meta: &PortMetadata,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    _total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    // 1. Extract room_id
    let room_id = step.room_id.ok_or_else(|| {
        HubError::Internal(anyhow!(
            "step {} has execution_mode='room' but no room_id",
            step.id
        ))
    })?;

    // 2. Check if this is a resume with a completed session
    if let Some(existing_output) = completed.get(&step.id) {
        if let Some(ref structured) = existing_output.structured_output {
            if structured.get("status").and_then(|v| v.as_str()) == Some("awaiting_room") {
                // This step was paused for user interaction — check if session is completed
                if let Some(session_id_str) =
                    structured.get("room_session_id").and_then(|v| v.as_str())
                {
                    if let Ok(session_id) = session_id_str.parse::<Uuid>() {
                        let room_repo = &state.repos().rooms;
                        if let Ok(Some(session)) = room_repo.get_room_session(session_id).await {
                            if session.status == "completed" {
                                // Session completed — extract outputs from transcript
                                info!(
                                    step_id = %step.id,
                                    session_id = %session_id,
                                    "Resuming room step — extracting outputs from completed session"
                                );
                                let transcript = room_repo
                                    .get_room_transcript(session_id)
                                    .await
                                    .unwrap_or_default();

                                let resolved_key =
                                    resolve_output_key(step, &port_meta.step_outputs);
                                let key_ref = if resolved_key.is_empty() {
                                    None
                                } else {
                                    Some(resolved_key.as_str())
                                };
                                let (envelope_data, output) =
                                    extract_room_outputs_from_transcript(&transcript, key_ref);

                                if !output.variable_name.is_empty() {
                                    if let Some(ref structured) = output.structured_output {
                                        var_outputs.insert(
                                            output.variable_name.clone(),
                                            structured.clone(),
                                        );
                                    }
                                }

                                let envelope = StepExecutionEnvelope {
                                    status: ExecutionStatus::Success,
                                    data: Some(envelope_data),
                                    metadata: ExecutionMetadata {
                                        execution_id: session_id,
                                        execution_time_ms: 0,
                                        tokens_in: None,
                                        tokens_out: None,
                                        cost_usd: None,
                                        model: None,
                                        agent_id: None,
                                        iteration_index: None,
                                        iteration_label: None,
                                        routing_label: None,
                                        selected_routing_document_id: None,
                                        upstream_agent_id: None,
                                        upstream_routing_label: None,
                                        room_session_id: Some(session_id),
                                        room_id: Some(room_id),
                                        total_rounds: Some(session.current_turn),
                                    },
                                    error: None,
                                };
                                completed_envelopes.insert(step.id, envelope);
                                completed.insert(step.id, output);
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Load room configuration
    let room_repo = &state.repos().rooms;
    let room = room_repo
        .get_room(room_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load room: {}", e)))?
        .ok_or_else(|| HubError::Internal(anyhow!("room {} not found", room_id)))?;

    // 4. Load members and their agents
    let members = room_repo
        .list_room_members(room_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load room members: {}", e)))?;

    let mut members_with_agents: Vec<RoomMemberWithAgent> = Vec::new();
    for member in members {
        let agent = state
            .repo()
            .get_persisted_agent(member.agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: step.id,
                agent_id: member.agent_id,
            })?;
        members_with_agents.push(RoomMemberWithAgent { member, agent });
    }

    // 5. Resolve port inputs
    let port_inputs = if let Some(inputs) = port_meta.step_inputs.get(&step.id) {
        match resolve_port_inputs(
            step.id,
            edges,
            inputs,
            &port_meta.step_outputs,
            completed_envelopes,
        ) {
            Ok(resolved) => {
                debug!(step_id = %step.id, ports = resolved.len(), "Resolved port inputs for room step");
                Some(resolved)
            }
            Err(e) => {
                warn!("Port resolution failed for room step {}: {}", step.id, e);
                None
            }
        }
    } else {
        None
    };

    // 6. Compose initial prompt
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

    // 7. Create room session
    let session = room_repo
        .create_room_session(room_id, Some(ctx.run_id))
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to create room session: {}", e)))?;

    // Broadcast: step started (room step)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step
                .output_variable_name
                .clone()
                .unwrap_or_else(|| step.id.to_string()),
            agent_id: None,
            execution_id: Some(session.id),
        },
    );

    info!(
        step_id = %step.id,
        room_id = %room_id,
        session_id = %session.id,
        "Starting room step execution"
    );

    // 8. Get LLM provider
    let provider = engine.provider();

    // 9. Check execution mode
    let interactive = step.agent_execution_mode.as_deref() == Some("interactive");

    if interactive {
        // ── Interactive path: run initial round, then pause for user ──

        let user_message = build_dag_room_prompt(&prompt, 0, room.max_turns);
        let turn_result = execute_room_turn(
            state,
            provider,
            &room,
            &session,
            &members_with_agents,
            &user_message,
            ctx.user_id,
            cancel,
        )
        .await?;

        for speaker in &turn_result.speakers {
            *total_input_tokens += speaker.input_tokens as i64;
            *total_output_tokens += speaker.output_tokens as i64;
        }

        // Store partial output for resume detection
        let partial = StepOutput {
            variable_name: resolve_output_key(step, &port_meta.step_outputs),
            raw_output: format!(
                "{{\"room_session_id\":\"{}\",\"status\":\"awaiting_room\"}}",
                session.id
            ),
            structured_output: Some(serde_json::json!({
                "room_session_id": session.id.to_string(),
                "status": "awaiting_room"
            })),
        };
        completed.insert(step.id, partial);

        // Broadcast: step paused (awaiting user interaction)
        broadcast_workflow_event(
            state,
            ctx,
            step.workflow_id,
            WorkflowEventKind::StepPaused {
                step_id: step.id,
                step_name: step
                    .output_variable_name
                    .clone()
                    .unwrap_or_else(|| step.id.to_string()),
            },
        );

        info!(
            step_id = %step.id,
            session_id = %session.id,
            "Room step paused — awaiting user interaction"
        );

        return Err(HubError::AwaitingUser {
            step_id: step.id,
            execution_id: session.id,
        });
    }

    // ── Auto-run path: execute all rounds ──

    let mut last_turn_result = None;
    let mut current_session = session.clone();

    for round in 0..room.max_turns {
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        let user_message = build_dag_room_prompt(&prompt, round, room.max_turns);
        let turn_result = execute_room_turn(
            state,
            provider.clone(),
            &room,
            &current_session,
            &members_with_agents,
            &user_message,
            ctx.user_id,
            cancel,
        )
        .await?;

        for speaker in &turn_result.speakers {
            *total_input_tokens += speaker.input_tokens as i64;
            *total_output_tokens += speaker.output_tokens as i64;
        }

        let session_done = turn_result.session_completed;
        last_turn_result = Some(turn_result);
        if session_done {
            break;
        }

        // Reload session for updated turn counter
        if let Ok(Some(updated)) = room_repo.get_room_session(current_session.id).await {
            current_session = updated;
        }
    }

    // 10. Extract per-agent outputs from final turn
    let room_output_key = resolve_output_key(step, &port_meta.step_outputs);
    let room_key_ref = if room_output_key.is_empty() {
        None
    } else {
        Some(room_output_key.as_str())
    };
    let (envelope_data, output) = if let Some(ref final_turn) = last_turn_result {
        extract_room_outputs_from_speakers(&final_turn.speakers, room_key_ref)
    } else {
        // No rounds executed (max_turns = 0)
        (
            JsonValue::Object(serde_json::Map::new()),
            StepOutput {
                variable_name: room_output_key.clone(),
                raw_output: "{}".to_string(),
                structured_output: Some(JsonValue::Object(serde_json::Map::new())),
            },
        )
    };

    // 11. Store results
    if !output.variable_name.is_empty() {
        if let Some(ref structured) = output.structured_output {
            var_outputs.insert(output.variable_name.clone(), structured.clone());
        }
    }

    let final_turn_number = last_turn_result
        .as_ref()
        .map(|t| t.turn_number)
        .unwrap_or(0);
    let envelope = StepExecutionEnvelope {
        status: ExecutionStatus::Success,
        data: Some(envelope_data),
        metadata: ExecutionMetadata {
            execution_id: session.id,
            execution_time_ms: 0,
            tokens_in: Some(*total_input_tokens as i32),
            tokens_out: Some(*total_output_tokens as i32),
            cost_usd: None,
            model: None,
            agent_id: None,
            iteration_index: None,
            iteration_label: None,
            routing_label: None,
            selected_routing_document_id: None,
            upstream_agent_id: None,
            upstream_routing_label: None,
            room_session_id: Some(session.id),
            room_id: Some(room_id),
            total_rounds: Some(final_turn_number),
        },
        error: None,
    };
    completed_envelopes.insert(step.id, envelope);
    completed.insert(step.id, output);

    // Broadcast: step completed (room step)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step
                .output_variable_name
                .clone()
                .unwrap_or_else(|| step.id.to_string()),
            agent_id: None,
            output: None,
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
        },
    );

    info!(
        step_id = %step.id,
        session_id = %session.id,
        rounds = final_turn_number,
        "Room step execution completed"
    );

    Ok(())
}

/// Extract per-agent outputs from SpeakerResult records (auto-run path).
pub(crate) fn extract_room_outputs_from_speakers(
    speakers: &[crate::server::executors::room::SpeakerResult],
    variable_name: Option<&str>,
) -> (JsonValue, StepOutput) {
    let mut composite = serde_json::Map::new();
    for speaker in speakers {
        let key = format!("agent:{}", speaker.agent_id);
        let value: JsonValue = serde_json::from_str(&speaker.content)
            .unwrap_or_else(|_| JsonValue::String(speaker.content.clone()));
        composite.insert(key, value);
    }
    let envelope_data = JsonValue::Object(composite);

    let output = StepOutput {
        variable_name: variable_name.unwrap_or_default().to_string(),
        raw_output: serde_json::to_string(&envelope_data).unwrap_or_default(),
        structured_output: Some(envelope_data.clone()),
    };

    (envelope_data, output)
}

/// Extract per-agent outputs from a room transcript (resume path).
///
/// Groups transcript entries by agent, takes each agent's last assistant message.
fn extract_room_outputs_from_transcript(
    transcript: &[crate::db::RoomTranscriptEntry],
    variable_name: Option<&str>,
) -> (JsonValue, StepOutput) {
    let mut last_by_agent: HashMap<String, String> = HashMap::new();

    for entry in transcript {
        // Transcript entries include all messages; we want the last content per agent
        last_by_agent.insert(entry.agent_name.clone(), entry.content.clone());
    }

    let mut composite = serde_json::Map::new();
    for (agent_name, content) in &last_by_agent {
        let key = agent_name.to_lowercase().replace(' ', "_");
        let value: JsonValue =
            serde_json::from_str(content).unwrap_or_else(|_| JsonValue::String(content.clone()));
        composite.insert(key, value);
    }
    let envelope_data = JsonValue::Object(composite);

    let output = StepOutput {
        variable_name: variable_name.unwrap_or_default().to_string(),
        raw_output: serde_json::to_string(&envelope_data).unwrap_or_default(),
        structured_output: Some(envelope_data.clone()),
    };

    (envelope_data, output)
}

/// Execute a for-each step: expand into N iterations, run sequentially.
///
/// Supports label-based routing: when `routing_mode = "label"`, each element
/// is routed to a different agent based on the value of `routing_field`.
async fn execute_for_each_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    _steps: &[WorkflowStepRow],
    _edges: &[WorkflowStepEdgeRow],
    var_outputs: &mut HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    completed_envelopes: &mut HashMap<Uuid, StepExecutionEnvelope>,
    port_meta: &PortMetadata,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    let for_each_ref = step
        .for_each_ref
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("for_each step {} missing for_each_ref", step.id))?;

    let array =
        resolve_for_each_array(for_each_ref, var_outputs, &ctx.prior_outputs).ok_or_else(|| {
            HubError::ForEachNotArray {
                reference: for_each_ref.to_string(),
            }
        })?;

    let label_field = step.for_each_label_field.as_deref();
    let routing_rules = port_meta.routing_rules.get(&step.id);
    let is_label_routing = step.routing_mode.as_deref() == Some("label") && routing_rules.is_some();

    // Broadcast: step started (for-each)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step
                .output_variable_name
                .clone()
                .unwrap_or_else(|| step.id.to_string()),
            agent_id: Some(agent.id),
            execution_id: None,
        },
    );

    info!(
        step_id = %step.id,
        count = array.len(),
        label_routing = is_label_routing,
        "for_each expansion"
    );

    let total_iterations = array.len();
    let mut iteration_outputs = Vec::new();
    // Cache loaded agents to avoid redundant DB calls
    let mut agent_cache: HashMap<Uuid, AgentRow> = HashMap::new();
    agent_cache.insert(agent.id, agent.clone());

    for (idx, element) in array.iter().enumerate() {
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }
        let label = extract_for_each_label(element, label_field);

        // Determine which agent to use for this iteration
        let iteration_agent = if is_label_routing {
            let routed_agent_id = label.as_ref().and_then(|lbl| {
                routing_rules
                    .unwrap()
                    .iter()
                    .find(|r| r.label_value == *lbl)
                    .map(|r| r.agent_id)
            });

            if let Some(routed_id) = routed_agent_id {
                if routed_id == agent.id {
                    agent
                } else if let Some(cached) = agent_cache.get(&routed_id) {
                    cached
                } else {
                    // Load routed agent from DB
                    match state.repo().get_persisted_agent(routed_id).await {
                        Ok(Some(routed_agent)) => {
                            agent_cache.insert(routed_id, routed_agent);
                            agent_cache.get(&routed_id).unwrap()
                        }
                        _ => {
                            warn!(
                                "Routed agent {} not found for label {:?}, falling back to default",
                                routed_id, label
                            );
                            agent
                        }
                    }
                }
            } else {
                debug!(label = ?label, "No routing rule matched, using default agent");
                agent
            }
        } else {
            agent
        };

        let prompt = compose_prompt(
            step,
            state.prompt_template_repo().as_deref(),
            state.doc_repo().as_deref(),
            state.workflow_repo().as_deref(),
            &**state.repo(),
            var_outputs,
            &ctx.prior_outputs,
            Some(element),
            None,
        )
        .await;

        // Create container for this iteration if configured (with optional VPN sidecar)
        let iter_container = create_optional_container(
            ctx.container_config.as_ref(),
            ctx.wg_client.as_deref(),
            "for-each-iter",
        )
        .await?;

        let result = run_with_vpn_watchdog(
            &iter_container,
            run_step_via_engine(
                engine,
                state,
                ctx,
                step,
                iteration_agent,
                &prompt,
                &port_meta.step_outputs,
                cancel,
                iter_container.as_ref().map(|mc| &mc.agent_handle),
            ),
        )
        .await;

        destroy_optional_container(&iter_container, ctx.wg_client.as_deref()).await;

        match result {
            Ok((output, in_tok, out_tok, cost)) => {
                *total_input_tokens += in_tok;
                *total_output_tokens += out_tok;
                *total_cost_usd += cost;
                iteration_outputs.push(output.structured_output.clone());

                // Broadcast: for-each progress
                broadcast_workflow_event(
                    state,
                    ctx,
                    step.workflow_id,
                    WorkflowEventKind::ForEachProgress {
                        step_id: step.id,
                        step_name: step
                            .output_variable_name
                            .clone()
                            .unwrap_or_else(|| step.id.to_string()),
                        completed: idx + 1,
                        total: total_iterations,
                    },
                );
            }
            Err(e) => {
                error!(
                    "for_each iteration {} failed for step {}: {}",
                    idx, step.id, e
                );
            }
        }
    }

    // Aggregate outputs as array
    let aggregated = JsonValue::Array(iteration_outputs.into_iter().flatten().collect());
    let variable_name = resolve_output_key(step, &port_meta.step_outputs);

    let output = StepOutput {
        variable_name: variable_name.clone(),
        structured_output: Some(aggregated.clone()),
        raw_output: String::new(),
    };

    // Store in var_outputs for downstream variable resolution
    if !variable_name.is_empty() {
        var_outputs.insert(variable_name, aggregated);
    }

    // Store envelope for downstream port resolution
    let envelope = wrap_in_envelope(&output, agent, step.id, 0, 0, 0.0);
    completed_envelopes.insert(step.id, envelope);

    completed.insert(step.id, output);

    // Broadcast: step completed (for-each)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step
                .output_variable_name
                .clone()
                .unwrap_or_else(|| step.id.to_string()),
            agent_id: Some(agent.id),
            output: None,
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
        },
    );

    Ok(())
}

/// Run a single step execution through the ExecutionEngine.
///
/// Creates the agent_execution record, builds a DagStepStrategy, and
/// calls `engine.execute()`. Returns (StepOutput, input_tokens, output_tokens, cost).
async fn run_step_via_engine(
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

    // Resolve mode (no context hint initially - future: pass step description)
    let mode = if let Some(resolver) = state.mode_resolver() {
        resolver
            .resolve(agent, prompt, None)
            .await
            .map_err(|e| HubError::Internal(anyhow!("Mode resolution failed: {}", e)))?
    } else {
        // Fallback: construct agent defaults for backward compatibility
        construct_agent_defaults(agent, state.repo())
            .await
            .map_err(HubError::Internal)?
    };

    // Build system prompt: mode result + schema enforcement
    let mut system_prompt = mode.system_prompt; // agent + mode already merged
    let mut output_schema_value: Option<JsonValue> = None;
    if let Some(schema_id) = step.output_schema_id {
        let os_repo = &state.repos().output_schemas;
        if let Ok(Some(schema)) = os_repo.get_output_schema(schema_id).await {
            system_prompt.push_str(&format!(
                "\n\nYou MUST respond with valid JSON matching this schema:\n```json\n{}\n```\nRespond ONLY with the JSON object, no other text.",
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
            mode.selected_mode_id, // Track which mode was used
            None,
            None,
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
        tools: mode.tools,             // Use mode tools
        tool_names: mode.tool_names,   // Use mode tool names
        temperature: mode.temperature, // Use mode temperature
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

    let mut filters: Vec<Arc<dyn ExecutionFilter>> =
        vec![Arc::new(AgentGuidanceFilter::new(state.repo().clone()))];

    if let Some(ae_repo) = state.agent_execution_repo() {
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
    let structured = super::strategies::dag_step::DagStepStrategy::parse_output(&result.content);

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

// ============================================================================
// Phase 6B: Chained For-Each Pipeline Execution
// ============================================================================

/// Result of executing one item through all stages of a for-each chain.
struct PipelineItemResult {
    index: usize,
    #[allow(dead_code)]
    label: Option<String>,
    /// One envelope per stage, ordered by chain stage index.
    stage_envelopes: Vec<(Uuid, StepExecutionEnvelope)>,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
}

/// Data needed for one stage of the pipeline, pre-loaded before task spawning.
#[derive(Clone)]
struct PipelineStageData {
    step: WorkflowStepRow,
    default_agent: AgentRow,
    routing_rules: Option<Vec<StepRoutingRuleRow>>,
    /// Pre-loaded routed agents for label routing.
    agent_cache: HashMap<Uuid, AgentRow>,
}

/// Execute a chain of for-each steps using per-item pipeline streaming.
///
/// For chain `[A, B]` with N items:
/// - Spawns N pipeline tasks, one per item index
/// - Each pipeline task runs: A[i] → B[i] sequentially
/// - All N pipelines run concurrently via JoinSet
/// - Collects results into aggregates for each step in the chain
#[allow(clippy::too_many_arguments)]
async fn execute_for_each_chain(
    _engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    chain: &ForEachChain,
    step_map: &HashMap<Uuid, &WorkflowStepRow>,
    _edges: &[WorkflowStepEdgeRow],
    var_outputs: &mut HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    completed_envelopes: &mut HashMap<Uuid, StepExecutionEnvelope>,
    port_meta: &PortMetadata,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    // 1. Get the first step and resolve the for-each array
    let first_step = step_map
        .get(&chain.step_ids[0])
        .ok_or_else(|| HubError::Internal(anyhow!("chain head step not found")))?;

    let for_each_ref = first_step.for_each_ref.as_deref().ok_or_else(|| {
        anyhow::anyhow!("chain head step {} missing for_each_ref", chain.step_ids[0])
    })?;

    let array =
        resolve_for_each_array(for_each_ref, var_outputs, &ctx.prior_outputs).ok_or_else(|| {
            HubError::ForEachNotArray {
                reference: for_each_ref.to_string(),
            }
        })?;

    info!(
        chain_len = chain.step_ids.len(),
        items = array.len(),
        head_step = %chain.step_ids[0],
        "Starting chained for-each pipeline"
    );

    // 2. Pre-load all stage data (agents + routing rules)
    let mut stages: Vec<PipelineStageData> = Vec::with_capacity(chain.step_ids.len());

    for step_id in &chain.step_ids {
        let step = step_map
            .get(step_id)
            .ok_or_else(|| HubError::Internal(anyhow!("chain step {} not found", step_id)))?;

        let default_agent = state
            .repo()
            .get_persisted_agent(step.agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: *step_id,
                agent_id: step.agent_id,
            })?;

        let routing_rules = port_meta.routing_rules.get(step_id).cloned();

        // Pre-load routed agents into cache
        let mut agent_cache: HashMap<Uuid, AgentRow> = HashMap::new();
        agent_cache.insert(default_agent.id, default_agent.clone());

        if let Some(ref rules) = routing_rules {
            for rule in rules {
                use std::collections::hash_map::Entry;
                if let Entry::Vacant(entry) = agent_cache.entry(rule.agent_id) {
                    if let Ok(Some(routed_agent)) =
                        state.repo().get_persisted_agent(rule.agent_id).await
                    {
                        entry.insert(routed_agent);
                    }
                }
            }
        }

        stages.push(PipelineStageData {
            step: (*step).clone(),
            default_agent,
            routing_rules,
            agent_cache,
        });
    }

    // 3. Spawn per-item pipeline tasks
    let engine_provider = state
        .provider()
        .ok_or(HubError::ProviderNotConfigured)?
        .clone();
    let cancel_token = cancel.cloned();

    let mut join_set = tokio::task::JoinSet::new();

    for (idx, element) in array.iter().enumerate() {
        let stages_clone = stages.clone();
        let state_clone = state.clone();
        let ctx_clone = ctx.clone();
        let element_clone = element.clone();
        let cancel_clone = cancel_token.clone();
        let provider_clone = engine_provider.clone();
        let step_outputs_clone = port_meta.step_outputs.clone();

        join_set.spawn(async move {
            let task_engine = ExecutionEngine::new(provider_clone);
            execute_pipeline_item(
                &task_engine,
                &state_clone,
                &ctx_clone,
                &stages_clone,
                idx,
                &element_clone,
                &step_outputs_clone,
                cancel_clone.as_ref(),
            )
            .await
        });
    }

    // 4. Collect results
    let mut item_results: Vec<PipelineItemResult> = Vec::with_capacity(array.len());

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(item_result)) => {
                item_results.push(item_result);
            }
            Ok(Err(e)) => {
                error!("Pipeline item failed: {}", e);
                // Continue collecting other results — partial success
            }
            Err(join_err) => {
                error!("Pipeline task panicked: {}", join_err);
            }
        }
    }

    // Sort by index for deterministic ordering
    item_results.sort_by_key(|r| r.index);

    // 5. Build per-step aggregates
    for (stage_idx, step_id) in chain.step_ids.iter().enumerate() {
        let step = step_map.get(step_id).unwrap();
        let variable_name = resolve_output_key(step, &port_meta.step_outputs);

        // Collect iteration outputs for this stage
        let mut iteration_outputs: Vec<Option<JsonValue>> = Vec::with_capacity(array.len());
        let mut stage_input_tokens: i64 = 0;
        let mut stage_output_tokens: i64 = 0;
        let mut stage_cost: f32 = 0.0;

        for item in &item_results {
            if stage_idx < item.stage_envelopes.len() {
                let (_, ref envelope) = item.stage_envelopes[stage_idx];
                iteration_outputs.push(envelope.data.clone());
            } else {
                // Item failed before reaching this stage
                iteration_outputs.push(None);
            }
        }

        // Accumulate tokens only for the final stage (avoid double-counting)
        if stage_idx == chain.step_ids.len() - 1 {
            for item in &item_results {
                stage_input_tokens += item.input_tokens;
                stage_output_tokens += item.output_tokens;
                stage_cost += item.cost_usd;
            }
        }

        let aggregated = JsonValue::Array(iteration_outputs.into_iter().flatten().collect());

        let output = StepOutput {
            variable_name: variable_name.clone(),
            structured_output: Some(aggregated.clone()),
            raw_output: String::new(),
        };

        if !variable_name.is_empty() {
            var_outputs.insert(variable_name, aggregated);
        }

        let default_agent = &stages[stage_idx].default_agent;
        let envelope = wrap_in_envelope(
            &output,
            default_agent,
            *step_id,
            stage_input_tokens,
            stage_output_tokens,
            stage_cost,
        );
        completed_envelopes.insert(*step_id, envelope);
        completed.insert(*step_id, output);

        // Only accumulate totals once (at the end)
        if stage_idx == chain.step_ids.len() - 1 {
            *total_input_tokens += stage_input_tokens;
            *total_output_tokens += stage_output_tokens;
            *total_cost_usd += stage_cost;
        }
    }

    info!(
        chain_len = chain.step_ids.len(),
        items_completed = item_results.len(),
        "Chained for-each pipeline complete"
    );

    Ok(())
}

/// Execute one item through all stages of a pipeline chain sequentially.
///
/// This runs as a spawned Tokio task. Each stage's output becomes the input
/// for the next stage, with upstream routing context propagated forward.
#[allow(clippy::too_many_arguments)]
async fn execute_pipeline_item(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    stages: &[PipelineStageData],
    item_index: usize,
    element: &JsonValue,
    step_outputs: &HashMap<Uuid, Vec<StepOutputRow>>,
    cancel: Option<&CancellationToken>,
) -> Result<PipelineItemResult, HubError> {
    let mut stage_envelopes: Vec<(Uuid, StepExecutionEnvelope)> = Vec::with_capacity(stages.len());
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost: f32 = 0.0;

    // Track upstream context for propagation
    let mut upstream_agent_id: Option<Uuid> = None;
    let mut upstream_routing_label: Option<String> = None;

    // The current element being processed — starts as the original, then becomes
    // the previous stage's output data for subsequent stages.
    let mut current_element = element.clone();

    let label = extract_for_each_label(element, stages[0].step.for_each_label_field.as_deref());

    for (stage_idx, stage) in stages.iter().enumerate() {
        // Check cancellation
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        let step = &stage.step;

        // Determine which agent handles this item at this stage
        let iteration_agent = resolve_pipeline_agent(
            &current_element,
            element, // original element for routing field lookup
            step,
            &stage.routing_rules,
            &stage.default_agent,
            &stage.agent_cache,
        );

        let routing_label = if step.routing_mode.as_deref() == Some("label") {
            extract_for_each_label(element, step.routing_field.as_deref())
        } else {
            None
        };

        // Build prompt with the current element
        let prompt = compose_prompt(
            step,
            state.prompt_template_repo().as_deref(),
            state.doc_repo().as_deref(),
            state.workflow_repo().as_deref(),
            &**state.repo(),
            &HashMap::new(), // pipeline items don't use var_outputs
            &ctx.prior_outputs,
            Some(&current_element),
            None,
        )
        .await;

        // Append upstream context to prompt for stages > 0
        let prompt = if stage_idx > 0 {
            let mut p = prompt;
            if let Some(ref ua_id) = upstream_agent_id {
                p.push_str(&format!(
                    "\n\n## Upstream Context\nThis item was processed by agent {} (routing label: {}).\n",
                    ua_id,
                    upstream_routing_label.as_deref().unwrap_or("none")
                ));
            }
            p
        } else {
            prompt
        };

        // Create container for this pipeline stage if configured (with optional VPN sidecar)
        let stage_container = create_optional_container(
            ctx.container_config.as_ref(),
            ctx.wg_client.as_deref(),
            "pipeline-stage",
        )
        .await?;

        let result = run_with_vpn_watchdog(
            &stage_container,
            run_step_via_engine(
                engine,
                state,
                ctx,
                step,
                iteration_agent,
                &prompt,
                step_outputs,
                cancel,
                stage_container.as_ref().map(|mc| &mc.agent_handle),
            ),
        )
        .await;

        destroy_optional_container(&stage_container, ctx.wg_client.as_deref()).await;

        match result {
            Ok((output, in_tok, out_tok, cost)) => {
                total_input_tokens += in_tok;
                total_output_tokens += out_tok;
                total_cost += cost;

                let envelope = StepExecutionEnvelope {
                    status: ExecutionStatus::Success,
                    data: output.structured_output.clone(),
                    metadata: ExecutionMetadata {
                        execution_id: Uuid::new_v4(),
                        execution_time_ms: 0,
                        tokens_in: Some(in_tok as i32),
                        tokens_out: Some(out_tok as i32),
                        cost_usd: Some(cost as f64),
                        model: Some(iteration_agent.model_id.clone()),
                        agent_id: Some(iteration_agent.id),
                        iteration_index: Some(item_index),
                        iteration_label: label.clone(),
                        routing_label: routing_label.clone(),
                        selected_routing_document_id: None,
                        upstream_agent_id,
                        upstream_routing_label: upstream_routing_label.clone(),
                        room_session_id: None,
                        room_id: None,
                        total_rounds: None,
                    },
                    error: None,
                };

                // Propagate context for next stage
                upstream_agent_id = Some(iteration_agent.id);
                upstream_routing_label = routing_label;

                // Next stage uses this stage's output as its element
                if let Some(ref data) = output.structured_output {
                    current_element = data.clone();
                }

                stage_envelopes.push((step.id, envelope));
            }
            Err(e) => {
                error!(
                    "Pipeline item {} failed at stage {} (step {}): {}",
                    item_index, stage_idx, step.id, e
                );

                // Record error envelope for this stage
                let error_envelope = StepExecutionEnvelope {
                    status: ExecutionStatus::Error,
                    data: None,
                    metadata: ExecutionMetadata {
                        execution_id: Uuid::new_v4(),
                        execution_time_ms: 0,
                        tokens_in: None,
                        tokens_out: None,
                        cost_usd: None,
                        model: Some(iteration_agent.model_id.clone()),
                        agent_id: Some(iteration_agent.id),
                        iteration_index: Some(item_index),
                        iteration_label: label.clone(),
                        routing_label: None,
                        selected_routing_document_id: None,
                        upstream_agent_id,
                        upstream_routing_label: upstream_routing_label.clone(),
                        room_session_id: None,
                        room_id: None,
                        total_rounds: None,
                    },
                    error: Some(crate::types::ExecutionError {
                        message: format!("{}", e),
                        error_type: "PipelineStageError".to_string(),
                        retryable: false,
                        details: None,
                    }),
                };

                stage_envelopes.push((step.id, error_envelope));

                // Don't continue to next stages — item failed
                break;
            }
        }
    }

    Ok(PipelineItemResult {
        index: item_index,
        label,
        stage_envelopes,
        input_tokens: total_input_tokens,
        output_tokens: total_output_tokens,
        cost_usd: total_cost,
    })
}

/// Resolve which agent should handle an item at a pipeline stage.
///
/// Uses the ORIGINAL element (from the planner output) for routing field lookup,
/// so that both stages can route by the same field (e.g., "category").
fn resolve_pipeline_agent<'a>(
    _current_element: &JsonValue,
    original_element: &JsonValue,
    step: &WorkflowStepRow,
    routing_rules: &Option<Vec<StepRoutingRuleRow>>,
    default_agent: &'a AgentRow,
    agent_cache: &'a HashMap<Uuid, AgentRow>,
) -> &'a AgentRow {
    let is_label_routing = step.routing_mode.as_deref() == Some("label") && routing_rules.is_some();

    if !is_label_routing {
        return default_agent;
    }

    let routing_field = step.routing_field.as_deref().unwrap_or("category");
    let label = original_element.get(routing_field).and_then(|v| v.as_str());

    if let Some(label_str) = label {
        let routed_agent_id = routing_rules
            .as_ref()
            .unwrap()
            .iter()
            .find(|r| r.label_value == label_str)
            .map(|r| r.agent_id);

        if let Some(routed_id) = routed_agent_id {
            if let Some(agent) = agent_cache.get(&routed_id) {
                return agent;
            }
        }
    }

    default_agent
}

/// Resume a workflow DAG from a paused state after an interactive step is approved.
///
/// Reconstructs the completed step outputs from agent_executions in the DB,
/// injects the approved output for the paused step, then continues executing
/// downstream steps.
pub async fn resume_workflow_via_engine(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    pre_completed: HashMap<Uuid, StepOutput>,
    pre_var_outputs: HashMap<String, JsonValue>,
    cancel: Option<&CancellationToken>,
) -> Result<WorkflowExecutionResult, HubError> {
    let sorted = topological_sort(steps, edges).map_err(|_| HubError::DagCycle)?;
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

    let mut completed = pre_completed;
    let mut var_outputs = pre_var_outputs;
    let mut completed_envelopes: HashMap<Uuid, StepExecutionEnvelope> = HashMap::new();
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f32 = 0.0;

    // Build synthetic envelopes for pre-completed steps
    for (step_id, output) in &completed {
        let envelope = StepExecutionEnvelope {
            status: if output.structured_output.is_some() {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Error
            },
            data: output.structured_output.clone(),
            metadata: ExecutionMetadata {
                execution_id: *step_id,
                execution_time_ms: 0,
                tokens_in: None,
                tokens_out: None,
                cost_usd: None,
                model: None,
                agent_id: None,
                iteration_index: None,
                iteration_label: None,
                routing_label: None,
                selected_routing_document_id: None,
                upstream_agent_id: None,
                upstream_routing_label: None,
                room_session_id: None,
                room_id: None,
                total_rounds: None,
            },
            error: None,
        };
        completed_envelopes.insert(*step_id, envelope);
    }

    // Pre-fetch port metadata
    let port_meta = prefetch_port_metadata(state, steps).await;

    // Phase 6B: Detect chained for-each pipelines
    let chains = detect_for_each_chains(steps, edges);
    let chain_member_set: HashSet<Uuid> = chains
        .iter()
        .flat_map(|c| c.step_ids.iter().copied())
        .collect();

    for step_id in &sorted {
        // Skip already-completed steps
        if completed.contains_key(step_id) {
            continue;
        }

        let step = match step_map.get(step_id) {
            Some(s) => *s,
            None => continue,
        };

        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        let parents = get_parent_steps(*step_id, edges);
        let all_parents_done = parents.iter().all(|pid| completed.contains_key(pid));
        if !all_parents_done {
            warn!("Step {} has uncompleted parents, skipping", step_id);
            continue;
        }

        // Phase 6B: If this is the head of a for-each chain, execute the whole chain
        if let Some(chain) = chains.iter().find(|c| c.step_ids[0] == *step_id) {
            execute_for_each_chain(
                engine,
                state,
                ctx,
                chain,
                &step_map,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
            continue;
        }

        // Skip non-head chain members (already executed by chain head)
        if chain_member_set.contains(step_id) {
            continue;
        }

        let agent = state
            .repo()
            .get_persisted_agent(step.agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: *step_id,
                agent_id: step.agent_id,
            })?;

        if step.execution_mode == "room" {
            execute_room_step(
                engine,
                state,
                ctx,
                step,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
        } else if step.execution_mode == "cavernous" {
            execute_cavernous_step(
                engine,
                state,
                ctx,
                step,
                &agent,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
        } else if step.execution_mode == "for_each" {
            execute_for_each_step(
                engine,
                state,
                ctx,
                step,
                &agent,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
        } else {
            execute_single_step(
                engine,
                state,
                ctx,
                step,
                &agent,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
        }
    }

    let final_outputs: HashMap<String, StepOutput> = completed
        .into_iter()
        .map(|(id, out)| (id.to_string(), out))
        .collect();

    Ok(WorkflowExecutionResult {
        outputs: final_outputs,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
    })
}

/// Orchestrator: resume the DAG after an interactive step is approved.
///
/// Finds the paused workflow execution, reconstructs completed state from
/// agent_executions in the DB, then continues executing downstream steps.
pub async fn resume_dag_from_approval(
    state: &AppState,
    paused_step_id: Uuid,
    approved_output: StepOutput,
) -> Result<(), HubError> {
    let wf_repo = &state.repos().workflows;
    let ae_repo = &state.repos().agent_executions;

    // Load the step to get workflow_id
    let step = wf_repo
        .get_step(paused_step_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("step load failed: {}", e)))?
        .ok_or_else(|| HubError::Internal(anyhow!("step {} not found", paused_step_id)))?;

    // Load workflow steps and edges
    let steps = wf_repo
        .list_steps(step.workflow_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("steps load failed: {}", e)))?;
    let edges = wf_repo
        .list_edges(step.workflow_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("edges load failed: {}", e)))?;

    let port_meta = prefetch_port_metadata(state, &steps).await;

    // Find the paused workflow_execution via agent_executions
    let step_ids: Vec<Uuid> = steps.iter().map(|s| s.id).collect();
    let completed_executions = ae_repo
        .list_completed_executions_for_step_ids(&step_ids)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load completed executions: {}", e)))?;

    let workflow_execution_id = completed_executions
        .iter()
        .find_map(|ae| ae.workflow_execution_id);

    let user_id = completed_executions
        .first()
        .map(|ae| ae.agent_id)
        .unwrap_or(Uuid::nil());

    // Reconstruct completed step outputs from DB
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();
    let mut completed: HashMap<Uuid, StepOutput> = HashMap::new();
    let mut var_outputs: HashMap<String, JsonValue> = HashMap::new();

    for ae in &completed_executions {
        if let Some(ae_step_id) = ae.workflow_step_id {
            if let Some(ws) = step_map.get(&ae_step_id) {
                let variable_name = resolve_output_key(ws, &port_meta.step_outputs);
                let output = StepOutput {
                    variable_name: variable_name.clone(),
                    raw_output: ae.output.clone().unwrap_or_default(),
                    structured_output: ae.structured_output.clone(),
                };
                if !variable_name.is_empty() {
                    if let Some(structured) = &output.structured_output {
                        var_outputs.insert(variable_name, structured.clone());
                    }
                }
                completed.insert(ae_step_id, output);
            }
        }
    }

    // Inject the approved step's output
    let approved_var_name = resolve_output_key(&step, &port_meta.step_outputs);
    let mut approved_output = approved_output;
    approved_output.variable_name = approved_var_name.clone();
    if !approved_var_name.is_empty() {
        if let Some(structured) = &approved_output.structured_output {
            var_outputs.insert(approved_var_name, structured.clone());
        }
    }
    completed.insert(paused_step_id, approved_output);

    // Broadcast: workflow resumed
    let workflow_id = step.workflow_id;

    // Build execution context
    let ctx = WorkflowExecutionContext {
        stage_execution_id: workflow_execution_id.unwrap_or(Uuid::new_v4()),
        run_id: Uuid::new_v4(),
        user_id,
        initial_input: String::new(),
        prior_outputs: HashMap::new(),
        execution_context: None,
        container_config: None,
        wg_client: None,
    };

    // Create engine and resume
    let provider = state
        .provider()
        .ok_or(HubError::ProviderNotConfigured)?
        .clone();
    let engine = ExecutionEngine::new(provider);

    // Broadcast: resumed
    broadcast_workflow_event(
        state,
        &ctx,
        workflow_id,
        WorkflowEventKind::Resumed {
            step_id: paused_step_id,
        },
    );

    info!(
        paused_step_id = %paused_step_id,
        completed_steps = completed.len(),
        remaining_steps = steps.len() - completed.len(),
        "Resuming DAG after interactive approval"
    );

    let result = resume_workflow_via_engine(
        &engine,
        state,
        &ctx,
        &steps,
        &edges,
        completed,
        var_outputs,
        None,
    )
    .await;

    // Update workflow_execution status if we know which one it is
    if let Some(wf_exec_id) = workflow_execution_id {
        if let Some(db) = state.db() {
            let coll_repo = crate::db::pg_repo::PgRepo::new(db.clone());
            match &result {
                Ok(wf_result) => {
                    let outputs_json: Option<JsonValue> = {
                        let mut map = serde_json::Map::new();
                        for (step_id_str, output) in &wf_result.outputs {
                            let val = output
                                .structured_output
                                .clone()
                                .unwrap_or_else(|| JsonValue::String(output.raw_output.clone()));
                            map.insert(step_id_str.clone(), val);
                        }
                        Some(JsonValue::Object(map))
                    };
                    let _ = coll_repo
                        .update_workflow_execution_status(
                            wf_exec_id,
                            "completed",
                            outputs_json,
                            None,
                        )
                        .await;

                    info!(
                        workflow_execution_id = %wf_exec_id,
                        "Resumed workflow execution completed"
                    );
                }
                Err(HubError::AwaitingUser { .. }) => {
                    info!(
                        workflow_execution_id = %wf_exec_id,
                        "Resumed workflow hit another interactive step, re-pausing"
                    );
                }
                Err(e) => {
                    // Broadcast: workflow failed
                    broadcast_workflow_event(
                        state,
                        &ctx,
                        workflow_id,
                        WorkflowEventKind::Failed {
                            error: format!("{}", e),
                        },
                    );

                    let _ = coll_repo
                        .update_workflow_execution_status(
                            wf_exec_id,
                            "failed",
                            None,
                            Some(format!("{}", e)),
                        )
                        .await;
                    error!(
                        workflow_execution_id = %wf_exec_id,
                        error = %e,
                        "Resumed workflow execution failed"
                    );
                }
            }
        }
    }

    result.map(|_| ())
}

// ============================================================================
// Phase 7: Cavernous Routing — Document-Based Dynamic Execution
// ============================================================================

/// Execute a cavernous routing step: search for routing config documents,
/// select the best config via LLM, parse it, execute subtasks, and aggregate.
#[allow(clippy::too_many_arguments)]
async fn execute_cavernous_step(
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
    // Broadcast: step started (cavernous)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step
                .output_variable_name
                .clone()
                .unwrap_or_else(|| step.id.to_string()),
            agent_id: Some(agent.id),
            execution_id: None,
        },
    );

    info!(step_id = %step.id, "Starting cavernous routing step");

    // ── Compose the base prompt (variable + port resolution) ──────────────
    let port_inputs = if let Some(inputs) = port_meta.step_inputs.get(&step.id) {
        match resolve_port_inputs(
            step.id,
            edges,
            inputs,
            &port_meta.step_outputs,
            completed_envelopes,
        ) {
            Ok(resolved) => Some(resolved),
            Err(e) => {
                warn!(
                    "Port resolution failed for cavernous step {}: {}",
                    step.id, e
                );
                None
            }
        }
    } else {
        None
    };

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

    // Append downstream routing context
    let mut prompt = prompt;
    let local_step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();
    let downstream_contexts =
        gather_downstream_routing_context(step.id, edges, &local_step_map, port_meta, state).await;
    for routing_ctx in &downstream_contexts {
        prompt.push_str(&build_routing_instruction_block(routing_ctx));
    }

    // ── Create parent agent_execution row ─────────────────────────────────
    let ae_repo = state
        .agent_execution_repo()
        .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;

    let parent_ae = ae_repo
        .create_agent_execution(
            agent.id,
            Some(step.id),
            false,
            None,
            &agent.system_prompt,
            &prompt,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to create cavernous agent execution: {}", e))?;

    // ── Phase 1: LLM generates search query ───────────────────────────────
    let cavernous_config = CavernousStepConfig {
        agent: agent.clone(),
        user_prompt: prompt.clone(),
        user_id: ctx.user_id,
        run_id: ctx.run_id,
    };
    let strategy = CavernousStepStrategy::new(cavernous_config);

    let sink = NullSink;
    let ae_repo_rec = state.agent_execution_repo();
    let tl_repo_rec = state.token_ledger_repo();
    let recorder = ExecutionRecorder::new(
        state.repo().as_ref(),
        ae_repo_rec.as_deref(),
        tl_repo_rec.as_deref(),
    );

    debug!(step_id = %step.id, "Cavernous Phase 1: generating search query");
    let phase1_result = engine
        .execute(&strategy, &prompt, &sink, &recorder, cancel)
        .await?;

    *total_input_tokens += phase1_result.input_tokens as i64;
    *total_output_tokens += phase1_result.output_tokens as i64;
    *total_cost_usd += compute_cost(
        &agent.model_id,
        phase1_result.input_tokens as i64,
        phase1_result.output_tokens as i64,
    );

    let search_query = strategy.search_query().await.ok_or_else(|| {
        HubError::Internal(anyhow!(
            "Cavernous Phase 1 failed: no search query generated"
        ))
    })?;

    info!(step_id = %step.id, query = %search_query, "Cavernous Phase 1 complete");

    // ── Programmatic: search routing documents ────────────────────────────
    let doc_repo = state
        .doc_repo()
        .ok_or_else(|| anyhow::anyhow!("doc_repo not configured"))?;

    let search_results = doc_repo
        .search_routing_documents(ctx.user_id, &search_query, 5)
        .await
        .map_err(|e| HubError::Internal(anyhow!("Routing document search failed: {}", e)))?;

    if search_results.is_empty() {
        return Err(HubError::Internal(anyhow!(
            "No routing documents found for query: {}",
            search_query
        )));
    }

    info!(
        step_id = %step.id,
        results = search_results.len(),
        "Found routing documents"
    );

    // ── Phase 2: LLM selects config ──────────────────────────────────────
    strategy.set_search_results(search_results.clone()).await;

    debug!(step_id = %step.id, "Cavernous Phase 2: selecting routing config");
    let phase2_result = engine
        .execute(&strategy, &prompt, &sink, &recorder, cancel)
        .await?;

    *total_input_tokens += phase2_result.input_tokens as i64;
    *total_output_tokens += phase2_result.output_tokens as i64;
    *total_cost_usd += compute_cost(
        &agent.model_id,
        phase2_result.input_tokens as i64,
        phase2_result.output_tokens as i64,
    );

    let selected_doc_id = strategy.selected_document_id().await.ok_or_else(|| {
        HubError::Internal(anyhow!("Cavernous Phase 2 failed: no config selected"))
    })?;

    info!(
        step_id = %step.id,
        selected_doc = %selected_doc_id,
        "Cavernous Phase 2 complete"
    );

    // ── Phase 3: Parse & validate routing config ─────────────────────────
    let selected_doc = doc_repo
        .get_document(selected_doc_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("Failed to fetch routing document: {}", e)))?
        .ok_or_else(|| {
            HubError::Internal(anyhow!(
                "Selected routing document {} not found",
                selected_doc_id
            ))
        })?;

    let routing_config: RoutingConfigDocument = serde_json::from_str(&selected_doc.content)
        .map_err(|e| {
            HubError::Internal(anyhow!(
                "Failed to parse routing config from document {}: {}",
                selected_doc_id,
                e
            ))
        })?;

    debug!(
        step_id = %step.id,
        strategy = %routing_config.strategy_name,
        subtasks = routing_config.subtasks.len(),
        "Parsed routing config"
    );

    // Store routing analysis
    let documents_found: Vec<DocumentSummary> = search_results
        .iter()
        .map(|r| DocumentSummary {
            id: r.id,
            title: r.title.clone(),
            description: r.summary.clone(),
            score: 0.0,
        })
        .collect();

    let routing_analysis = strategy
        .build_routing_analysis(documents_found)
        .await
        .ok_or_else(|| HubError::Internal(anyhow!("Failed to build routing analysis")))?;

    let analysis_json = serde_json::to_value(&routing_analysis)
        .map_err(|e| HubError::Internal(anyhow!("Failed to serialize routing analysis: {}", e)))?;

    let _ = ae_repo
        .update_agent_execution_routing(parent_ae.id, &analysis_json, Some(selected_doc_id))
        .await;

    // ── Phase 4: Execute subtasks ────────────────────────────────────────
    let layers = topo_sort_subtasks(&routing_config.subtasks)?;

    let mut subtask_outputs: HashMap<String, StepOutput> = HashMap::new();
    let subtask_order: Vec<String> = layers
        .iter()
        .flat_map(|layer: &Vec<&Subtask>| layer.iter().map(|s| s.id.clone()))
        .collect();

    for layer in &layers {
        if cancel.is_some_and(|c| c.is_cancelled()) {
            return Err(HubError::Internal(anyhow!("Cavernous step cancelled")));
        }

        // Execute subtasks within a layer in parallel (respecting max_parallel)
        let mut join_set = tokio::task::JoinSet::new();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(
            routing_config.max_parallel.max(1),
        ));

        for subtask in layer {
            // Resolve prompt template with input_mapping from completed subtask outputs
            let resolved_prompt = resolve_subtask_prompt(subtask, &subtask_outputs, &prompt);

            let engine_clone = engine.clone_with_provider();
            let state_clone = state.clone();
            let ctx_clone = ctx.clone();
            let step_clone = step.clone();
            let agent_id = subtask.agent_id;
            let parent_ae_id = parent_ae.id;
            let subtask_id = subtask.id.clone();
            let cancel_token = cancel.cloned();
            let sem = semaphore.clone();

            join_set.spawn(async move {
                let _permit = sem
                    .acquire()
                    .await
                    .map_err(|e| HubError::Internal(anyhow!("Semaphore error: {}", e)))?;

                let result = run_cavernous_subtask(
                    &engine_clone,
                    &state_clone,
                    &ctx_clone,
                    &step_clone,
                    agent_id,
                    &resolved_prompt,
                    parent_ae_id,
                    cancel_token.as_ref(),
                )
                .await;

                result.map(|r| (subtask_id, r))
            });
        }

        // Collect results from this layer
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok((subtask_id, (output, in_tok, out_tok, cost)))) => {
                    *total_input_tokens += in_tok;
                    *total_output_tokens += out_tok;
                    *total_cost_usd += cost;
                    subtask_outputs.insert(subtask_id, output);
                }
                Ok(Err(e)) => {
                    error!(step_id = %step.id, error = %e, "Subtask execution failed");
                    // Continue with other subtasks; partial results are OK
                }
                Err(e) => {
                    error!(step_id = %step.id, error = %e, "Subtask join error");
                }
            }
        }
    }

    // ── Phase 5: Aggregate results ───────────────────────────────────────
    let aggregated = aggregate_subtask_outputs(
        &subtask_outputs,
        &routing_config.aggregation_mode,
        &subtask_order,
    );

    let variable_name = resolve_output_key(step, &port_meta.step_outputs);
    let output = StepOutput {
        variable_name: variable_name.clone(),
        structured_output: Some(aggregated.clone()),
        raw_output: serde_json::to_string_pretty(&aggregated).unwrap_or_default(),
    };

    if !variable_name.is_empty() {
        var_outputs.insert(variable_name, aggregated);
    }

    // Build envelope for downstream port resolution
    let total_step_in = phase1_result.input_tokens as i64 + phase2_result.input_tokens as i64;
    let total_step_out = phase1_result.output_tokens as i64 + phase2_result.output_tokens as i64;
    let total_step_cost = compute_cost(&agent.model_id, total_step_in, total_step_out);
    let mut envelope = wrap_in_envelope(
        &output,
        agent,
        step.id,
        total_step_in,
        total_step_out,
        total_step_cost,
    );
    envelope.metadata.selected_routing_document_id = Some(selected_doc_id);
    completed_envelopes.insert(step.id, envelope);

    completed.insert(step.id, output);

    // Update parent execution status
    let _ = ae_repo
        .update_agent_execution_status(parent_ae.id, "completed", None, None)
        .await;

    // Broadcast: step completed (cavernous)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step
                .output_variable_name
                .clone()
                .unwrap_or_else(|| step.id.to_string()),
            agent_id: Some(agent.id),
            output: None,
            input_tokens: Some(*total_input_tokens as u64),
            output_tokens: Some(*total_output_tokens as u64),
            duration_ms: None,
        },
    );

    info!(
        step_id = %step.id,
        subtasks_completed = subtask_outputs.len(),
        "Cavernous routing step complete"
    );

    Ok(())
}

/// Resolve a subtask's prompt_template by substituting `{input.<subtask_id>.<path>}`
/// references with outputs from completed subtasks.
fn resolve_subtask_prompt(
    subtask: &Subtask,
    completed_outputs: &HashMap<String, StepOutput>,
    base_prompt: &str,
) -> String {
    let mut resolved = subtask.prompt_template.clone();

    // Replace {base_prompt} with the parent step's composed prompt
    resolved = resolved.replace("{base_prompt}", base_prompt);

    // Replace input_mapping references: {input.<dep_id>} or {input.<dep_id>.<path>}
    for (placeholder, source) in &subtask.input_mapping {
        let value: String = if let Some(output) = completed_outputs.get(source) {
            if let Some(ref structured) = output.structured_output {
                serde_json::to_string(structured).unwrap_or_default()
            } else {
                output.raw_output.clone()
            }
        } else {
            // Try resolving dot-path: "subtask_id.field.path"
            let parts: Vec<&str> = source.splitn(2, '.').collect();
            if parts.len() == 2 {
                if let Some(output) = completed_outputs.get(parts[0]) {
                    if let Some(ref structured) = output.structured_output {
                        resolve_dot_path(structured, parts[1])
                            .map(|v| {
                                if let Some(s) = v.as_str() {
                                    s.to_string()
                                } else {
                                    serde_json::to_string(&v).unwrap_or_default()
                                }
                            })
                            .unwrap_or_default()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        };

        let pattern = format!("{{{}}}", placeholder);
        resolved = resolved.replace(&pattern, &value);
    }

    resolved
}

/// Execute a single subtask within a cavernous routing step.
async fn run_cavernous_subtask(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    parent_step: &WorkflowStepRow,
    agent_id: Uuid,
    prompt: &str,
    parent_ae_id: Uuid,
    cancel: Option<&CancellationToken>,
) -> Result<(StepOutput, i64, i64, f32), HubError> {
    let agent = state
        .repo()
        .get_persisted_agent(agent_id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to load subtask agent: {}", e))?
        .ok_or_else(|| HubError::AgentNotFound {
            step_id: parent_step.id,
            agent_id,
        })?;

    // Resolve mode
    let mode = if let Some(resolver) = state.mode_resolver() {
        resolver
            .resolve(&agent, prompt, None)
            .await
            .map_err(|e| HubError::Internal(anyhow!("Mode resolution failed: {}", e)))?
    } else {
        construct_agent_defaults(&agent, state.repo())
            .await
            .map_err(HubError::Internal)?
    };

    let mut system_prompt = mode.system_prompt;
    if let Some(schema_id) = parent_step.output_schema_id {
        let os_repo = &state.repos().output_schemas;
        if let Ok(Some(schema)) = os_repo.get_output_schema(schema_id).await {
            system_prompt.push_str(&format!(
                "\n\nYou MUST respond with valid JSON matching this schema:\n```json\n{}\n```\nRespond ONLY with the JSON object, no other text.",
                serde_json::to_string_pretty(&schema.schema).unwrap_or_default()
            ));
        }
    }

    // Create agent_execution row linked to parent
    let ae_repo = state
        .agent_execution_repo()
        .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;

    let ae_row = ae_repo
        .create_agent_execution(
            agent.id,
            Some(parent_step.id),
            false,
            Some(parent_ae_id),
            &system_prompt,
            prompt,
            mode.selected_mode_id,
            None,
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to create subtask agent execution: {}", e))?;

    // Build strategy using existing DagStepStrategy
    let config = DagStepConfig {
        agent: agent.clone(),
        step: parent_step.clone(),
        system_prompt,
        user_prompt: prompt.to_string(),
        tools: mode.tools,
        tool_names: mode.tool_names,
        temperature: mode.temperature,
        execution_context: ctx.execution_context.clone(),
        container_handle: None, // Subtasks don't get their own container
        run_id: ctx.run_id,
        user_id: ctx.user_id,
        agent_execution_id: ae_row.id,
    };
    let strategy = DagStepStrategy::new(config, state.clone());

    let sink = NullSink;
    let ae_repo_for_recorder = state.agent_execution_repo();
    let tl_repo_for_recorder = state.token_ledger_repo();
    let recorder = ExecutionRecorder::new(
        state.repo().as_ref(),
        ae_repo_for_recorder.as_deref(),
        tl_repo_for_recorder.as_deref(),
    );

    let result = engine
        .execute(&strategy, prompt, &sink, &recorder, cancel)
        .await?;

    let cost = compute_cost(
        &agent.model_id,
        result.input_tokens as i64,
        result.output_tokens as i64,
    );

    let structured = super::strategies::dag_step::DagStepStrategy::parse_output(&result.content);

    let output = StepOutput {
        variable_name: String::new(), // Subtasks don't have variable names
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

/// Topological sort of subtasks into execution layers using Kahn's algorithm.
///
/// Returns layers where tasks within the same layer can execute in parallel,
/// and layers must execute sequentially (earlier layers before later ones).
pub(crate) fn topo_sort_subtasks(subtasks: &[Subtask]) -> Result<Vec<Vec<&Subtask>>, HubError> {
    if subtasks.is_empty() {
        return Ok(vec![]);
    }

    let id_to_subtask: HashMap<&str, &Subtask> =
        subtasks.iter().map(|s| (s.id.as_str(), s)).collect();

    // Build in-degree map and adjacency list
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for subtask in subtasks {
        in_degree.entry(subtask.id.as_str()).or_insert(0);
        for dep in &subtask.depends_on {
            let dep_str: &str = dep.as_str();
            if !id_to_subtask.contains_key(dep_str) {
                return Err(HubError::Internal(anyhow!(
                    "Subtask '{}' depends on unknown subtask '{}'",
                    subtask.id,
                    dep
                )));
            }
            *in_degree.entry(subtask.id.as_str()).or_insert(0) += 1;
            dependents
                .entry(dep.as_str())
                .or_default()
                .push(subtask.id.as_str());
        }
    }

    let mut layers: Vec<Vec<&Subtask>> = Vec::new();
    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();
    queue.sort(); // Deterministic ordering

    let mut processed = 0;

    while !queue.is_empty() {
        let mut next_queue: Vec<&str> = Vec::new();
        let mut layer: Vec<&Subtask> = Vec::new();

        for &id in &queue {
            layer.push(id_to_subtask[id]);
            processed += 1;

            if let Some(deps) = dependents.get(id) {
                for &dep_id in deps {
                    let deg = in_degree.get_mut(dep_id).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        next_queue.push(dep_id);
                    }
                }
            }
        }

        layer.sort_by_key(|s| &s.id); // Deterministic within layer
        layers.push(layer);
        next_queue.sort();
        queue = next_queue;
    }

    if processed != subtasks.len() {
        return Err(HubError::Internal(anyhow!(
            "Dependency cycle detected in subtasks"
        )));
    }

    Ok(layers)
}

/// Aggregate subtask outputs according to the specified mode.
///
/// Modes:
/// - `"all_outputs"`: Map of subtask_id → output
/// - `"final_output"`: Last subtask's output only
/// - `"merge"`: Shallow merge of all structured outputs into one object
pub(crate) fn aggregate_subtask_outputs(
    results: &HashMap<String, StepOutput>,
    aggregation_mode: &str,
    subtask_order: &[String],
) -> JsonValue {
    match aggregation_mode {
        "final_output" => {
            // Return the output of the last subtask in topo order
            for id in subtask_order.iter().rev() {
                if let Some(output) = results.get(id) {
                    return output
                        .structured_output
                        .clone()
                        .unwrap_or_else(|| JsonValue::String(output.raw_output.clone()));
                }
            }
            JsonValue::Null
        }
        "merge" => {
            // Shallow merge all structured outputs into one object
            let mut merged = serde_json::Map::new();
            for id in subtask_order {
                if let Some(output) = results.get(id) {
                    if let Some(JsonValue::Object(obj)) = &output.structured_output {
                        for (k, v) in obj {
                            merged.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
            JsonValue::Object(merged)
        }
        // "all_outputs" and any unknown mode
        _ => {
            let mut map = serde_json::Map::new();
            for id in subtask_order {
                if let Some(output) = results.get(id) {
                    let value = output
                        .structured_output
                        .clone()
                        .unwrap_or_else(|| JsonValue::String(output.raw_output.clone()));
                    map.insert(id.clone(), value);
                }
            }
            JsonValue::Object(map)
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
