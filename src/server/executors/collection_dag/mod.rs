//! Collection DAG executor for multi-tier workflow orchestration.
//!
//! This module implements the execution of workflow collections, which are DAGs of workflows.
//! Collections can execute workflows sequentially or in parallel (respecting dependencies).

use anyhow::{anyhow, Result};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::traits::{WorkflowCollectionRepo, WorkflowRepo};
use crate::db::{
    CollectionRunRow, CollectionWorkflowEdgeRow, CollectionWorkflowRow, WorkflowExecutionRow,
};
use crate::server::hub::dag::{broadcast_workflow_event, execute_workflow_via_engine};
use crate::server::hub::dag::{ContainerExecutionConfig, DagPaused, WorkflowExecutionContext};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;

/// Executor for workflow collections (DAG of workflows).
pub struct CollectionDagExecutor<R>
where
    R: WorkflowCollectionRepo + Send + Sync + 'static,
{
    collection_repo: Arc<R>,
    workflow_repo: Arc<dyn WorkflowRepo>,
    state: Arc<AppState>,
}

impl<R> CollectionDagExecutor<R>
where
    R: WorkflowCollectionRepo + Send + Sync + 'static,
{
    /// Create a new collection DAG executor.
    pub fn new(
        collection_repo: Arc<R>,
        workflow_repo: Arc<dyn WorkflowRepo>,
        state: Arc<AppState>,
    ) -> Self {
        Self {
            collection_repo,
            workflow_repo,
            state,
        }
    }

    /// Execute a workflow collection (DAG of workflows).
    pub async fn execute_collection(
        &self,
        collection_id: Uuid,
        user_id: Uuid,
    ) -> Result<CollectionRunRow> {
        // 0. Verify LLM provider is configured before starting execution
        if self.state.provider().is_none() {
            return Err(anyhow!("LLM provider not configured"));
        }

        // 1. Load collection
        let collection = self
            .collection_repo
            .get_collection(collection_id)
            .await?
            .ok_or_else(|| anyhow!("Collection not found: {}", collection_id))?;

        // 2. Load workflows and edges
        let collection_workflows = self
            .collection_repo
            .list_collection_workflows(collection_id)
            .await?;
        let edges = self
            .collection_repo
            .list_collection_edges(collection_id)
            .await?;

        // 3. Create collection_run row
        let run = self
            .collection_repo
            .create_collection_run(collection_id, user_id)
            .await?;

        // 4. Execute based on execution_mode
        let result = match collection.execution_mode.as_str() {
            "sequential" => {
                self.execute_collection_sequential(run.id, &collection_workflows, &edges, user_id)
                    .await
            }
            "parallel" => {
                self.execute_collection_parallel(run.id, &collection_workflows, &edges, user_id)
                    .await
            }
            _ => Err(anyhow!(
                "Unknown execution mode: {}",
                collection.execution_mode
            )),
        };

        // 5. Update collection_run status
        let final_run = match result {
            Ok(_) => {
                self.collection_repo
                    .update_collection_run_status(run.id, "completed", None)
                    .await?
            }
            Err(ref e)
                if e.downcast_ref::<crate::server::hub::dag::DagPaused>()
                    .is_some() =>
            {
                self.collection_repo
                    .update_collection_run_status(run.id, "paused", None)
                    .await?
            }
            Err(e) => {
                let error_msg = format!("{:#}", e);
                self.collection_repo
                    .update_collection_run_status(run.id, "failed", Some(error_msg))
                    .await?
            }
        };

        Ok(final_run)
    }

    /// Execute workflows sequentially (one-at-a-time).
    async fn execute_collection_sequential(
        &self,
        run_id: Uuid,
        collection_workflows: &[CollectionWorkflowRow],
        edges: &[CollectionWorkflowEdgeRow],
        user_id: Uuid,
    ) -> Result<()> {
        // Topological sort workflows
        let workflow_ids: Vec<Uuid> = collection_workflows
            .iter()
            .map(|cw| cw.workflow_id)
            .collect();
        let sorted_workflow_ids =
            topological_sort_workflows(&workflow_ids, collection_workflows, edges)?;

        let mut completed_workflows: HashMap<Uuid, WorkflowExecutionRow> = HashMap::new();

        for workflow_id in sorted_workflow_ids {
            // Collect outputs from completed workflows for variable resolution
            let prior_outputs =
                collect_workflow_outputs(&completed_workflows, &*self.workflow_repo).await?;

            // Execute workflow and capture outputs
            let workflow_exec = self
                .execute_workflow_in_collection(run_id, workflow_id, user_id, &prior_outputs)
                .await?;

            completed_workflows.insert(workflow_id, workflow_exec);
        }

        Ok(())
    }

    /// Execute workflows in parallel (respecting DAG dependencies).
    ///
    /// Spawns multiple workflow executions concurrently, respecting the DAG structure.
    /// Entry workflows (no dependencies) start immediately. Each workflow, upon completion,
    /// triggers its dependent workflows if all their dependencies are satisfied.
    async fn execute_collection_parallel(
        &self,
        run_id: Uuid,
        collection_workflows: &[CollectionWorkflowRow],
        edges: &[CollectionWorkflowEdgeRow],
        user_id: Uuid,
    ) -> Result<()> {
        use tokio::sync::RwLock;

        // Build dependency graph
        let workflow_ids: Vec<Uuid> = collection_workflows
            .iter()
            .map(|cw| cw.workflow_id)
            .collect();
        let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
        let mut children: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

        for &workflow_id in &workflow_ids {
            in_degree.insert(workflow_id, 0);
            children.insert(workflow_id, Vec::new());
        }

        for edge in edges {
            children
                .entry(edge.from_workflow_id)
                .or_default()
                .push(edge.to_workflow_id);
            *in_degree.entry(edge.to_workflow_id).or_default() += 1;
        }

        // Shared state for parallel execution
        let completed = Arc::new(RwLock::new(HashMap::<Uuid, WorkflowExecutionRow>::new()));
        let in_degree = Arc::new(RwLock::new(in_degree));
        let children = Arc::new(children);

        // Find entry workflows (no dependencies)
        let ready: Vec<Uuid> = {
            let deg = in_degree.read().await;
            deg.iter()
                .filter(|(_, &d)| d == 0)
                .map(|(&id, _)| id)
                .collect()
        };

        // Channel to collect errors from spawned tasks
        let (error_tx, mut error_rx) = tokio::sync::mpsc::channel::<anyhow::Error>(10);
        let any_paused = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Spawn entry workflows
        let mut handles = Vec::with_capacity(ready.len());
        for workflow_id in ready {
            let handle = self.spawn_workflow_with_cascade(
                run_id,
                workflow_id,
                user_id,
                Arc::clone(&completed),
                Arc::clone(&in_degree),
                Arc::clone(&children),
                error_tx.clone(),
                Arc::clone(&any_paused),
            );
            handles.push(handle);
        }

        // Drop the sender so the channel closes when all tasks complete
        drop(error_tx);

        // Wait for all workflows to complete
        let results = futures::future::join_all(handles).await;

        // Check for errors from spawned tasks
        let mut errors = Vec::new();
        while let Some(err) = error_rx.recv().await {
            errors.push(err);
        }

        // Check for panic in join handles
        for result in results {
            if let Err(e) = result {
                errors.push(anyhow::anyhow!("Workflow execution panicked: {}", e));
            }
        }

        if !errors.is_empty() {
            return Err(anyhow::anyhow!(
                "Parallel execution failed with {} error(s): {}",
                errors.len(),
                errors.first().unwrap()
            ));
        }

        if any_paused.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(crate::server::hub::dag::DagPaused {
                step_id: Uuid::nil(),
                execution_id: Uuid::nil(),
            }
            .into());
        }

        Ok(())
    }

    /// Spawn a workflow execution task that cascades to dependent workflows upon completion.
    #[allow(clippy::too_many_arguments)] // Concurrency primitives + IDs, no natural grouping
    fn spawn_workflow_with_cascade(
        &self,
        run_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
        completed: Arc<tokio::sync::RwLock<HashMap<Uuid, WorkflowExecutionRow>>>,
        in_degree: Arc<tokio::sync::RwLock<HashMap<Uuid, usize>>>,
        children: Arc<HashMap<Uuid, Vec<Uuid>>>,
        error_tx: tokio::sync::mpsc::Sender<anyhow::Error>,
        any_paused: Arc<std::sync::atomic::AtomicBool>,
    ) -> tokio::task::JoinHandle<()> {
        // Clone everything needed for the spawned task
        let collection_repo = Arc::clone(&self.collection_repo);
        let workflow_repo = Arc::clone(&self.workflow_repo);
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            // Collect prior workflow outputs
            let prior_outputs = {
                let comp = completed.read().await;
                match collect_workflow_outputs(&comp, workflow_repo.as_ref()).await {
                    Ok(outputs) => outputs,
                    Err(e) => {
                        let _ = error_tx.send(e).await;
                        return;
                    }
                }
            };

            // Execute this workflow
            let executor = CollectionDagExecutor {
                collection_repo: Arc::clone(&collection_repo),
                workflow_repo: Arc::clone(&workflow_repo),
                state: Arc::clone(&state),
            };

            let workflow_exec = match executor
                .execute_workflow_in_collection(run_id, workflow_id, user_id, &prior_outputs)
                .await
            {
                Ok(exec) => exec,
                Err(e)
                    if e.downcast_ref::<crate::server::hub::dag::DagPaused>()
                        .is_some() =>
                {
                    any_paused.store(true, std::sync::atomic::Ordering::Relaxed);
                    return; // don't cascade to children, don't send to error_tx
                }
                Err(e) => {
                    let _ = error_tx.send(e).await;
                    return;
                }
            };

            // Mark as completed
            {
                let mut comp = completed.write().await;
                comp.insert(workflow_id, workflow_exec);
            }

            // Decrement in_degree for children and spawn ready children
            let ready_children: Vec<Uuid> = {
                let mut deg = in_degree.write().await;
                let mut ready = Vec::new();

                if let Some(child_ids) = children.get(&workflow_id) {
                    for &child_id in child_ids {
                        if let Some(d) = deg.get_mut(&child_id) {
                            *d -= 1;
                            if *d == 0 {
                                ready.push(child_id);
                            }
                        }
                    }
                }

                ready
            };

            // Spawn ready children
            let mut child_handles = Vec::new();
            for child_id in ready_children {
                let handle = executor.spawn_workflow_with_cascade(
                    run_id,
                    child_id,
                    user_id,
                    Arc::clone(&completed),
                    Arc::clone(&in_degree),
                    Arc::clone(&children),
                    error_tx.clone(),
                    Arc::clone(&any_paused),
                );
                child_handles.push(handle);
            }

            // Wait for all children to complete
            let _ = futures::future::join_all(child_handles).await;
        })
    }

    /// Execute a single workflow within a collection run.
    ///
    /// Creates a workflow_execution record, loads the workflow steps/edges,
    /// executes the workflow DAG, and stores the outputs.
    async fn execute_workflow_in_collection(
        &self,
        collection_run_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
        prior_workflow_outputs: &HashMap<String, JsonValue>,
    ) -> Result<WorkflowExecutionRow> {
        // 1. Create workflow_execution record
        let workflow_exec = self
            .collection_repo
            .create_workflow_execution(collection_run_id, workflow_id, user_id)
            .await?;

        // 2. Update status to "running"
        let workflow_exec = self
            .collection_repo
            .update_workflow_execution_status(workflow_exec.id, "running", None, None)
            .await?;

        // 3. Load workflow steps and edges
        let steps = self.workflow_repo.list_steps(workflow_id).await?;
        let edges = self.workflow_repo.list_edges(workflow_id).await?;

        // 4. Create ExecutionEngine from centralized provider
        let engine = {
            let provider = self
                .state
                .provider()
                .ok_or_else(|| anyhow!("LLM provider not configured"))?
                .clone();
            ExecutionEngine::new(provider, self.state.env().debug_stream)
        };

        // 5. Build container config from workflow settings (if enabled)
        let container_config = {
            let wf_row = self
                .workflow_repo
                .get_workflow(workflow_id)
                .await?
                .ok_or_else(|| anyhow!("workflow {} not found", workflow_id))?;
            if wf_row.container_enabled {
                if let Some(repo_url) = &wf_row.target_repo_url {
                    let github_token = crate::execution::RedactedString::new(
                        self.state.env().github_token.clone().unwrap_or_default(),
                    );
                    Some(ContainerExecutionConfig {
                        clone_url: repo_url.clone(),
                        branch: wf_row.target_branch.clone(),
                        github_token,
                        image: None,
                        memory_limit: None,
                        cpu_limit: None,
                        vpn_enabled: wf_row.vpn_enabled,
                        workflow_id: None,
                        run_id: None,
                    })
                } else {
                    tracing::warn!(
                        workflow_id = %workflow_id,
                        "container_enabled is true but target_repo_url is missing"
                    );
                    None
                }
            } else {
                None
            }
        };

        // 6. Initialize wg-easy client if VPN is enabled
        let wg_client = if container_config.as_ref().is_some_and(|c| c.vpn_enabled) {
            match crate::execution::WgEasyConfig::from_env() {
                Some(cfg) => Some(std::sync::Arc::new(crate::execution::WgEasyClient::new(
                    cfg,
                ))),
                None => {
                    tracing::warn!(
                        workflow_id = %workflow_id,
                        "vpn_enabled is true but WGEASY_API_URL is not set"
                    );
                    None
                }
            }
        } else {
            None
        };

        // 7. Create execution context
        let ctx = WorkflowExecutionContext {
            stage_execution_id: workflow_exec.id,
            run_id: collection_run_id,
            user_id,
            initial_input: String::new(),
            prior_outputs: prior_workflow_outputs.clone(),
            execution_context: None,
            container_config,
            wg_client,
            snapshot: None,
        };

        // 6. Execute workflow DAG via unified hub engine
        let result =
            execute_workflow_via_engine(&engine, &self.state, &ctx, &steps, &edges, None).await;

        // 7. Handle result and update workflow_execution
        match result {
            Ok(wf_result) => {
                // Aggregate step outputs into workflow-level outputs
                let workflow_outputs = aggregate_step_outputs(&wf_result.outputs);

                // Update workflow_execution with outputs
                let workflow_exec = self
                    .collection_repo
                    .update_workflow_execution_status(
                        workflow_exec.id,
                        "completed",
                        Some(workflow_outputs.clone()),
                        None,
                    )
                    .await?;

                broadcast_workflow_event(
                    &self.state,
                    &ctx,
                    workflow_id,
                    WorkflowEventKind::Completed {
                        duration_ms: Some(wf_result.duration_ms),
                    },
                );

                Ok(workflow_exec)
            }
            Err(HubError::AwaitingUser {
                step_id,
                execution_id,
            }) => {
                let pause_metadata = serde_json::json!({
                    "__pause_state": {
                        "step_id": step_id.to_string(),
                        "execution_id": execution_id.to_string(),
                    }
                });
                self.collection_repo
                    .update_workflow_execution_status(
                        workflow_exec.id,
                        "paused",
                        Some(pause_metadata),
                        None,
                    )
                    .await?;
                // Propagate as DagPaused so collection-level pause detection still works
                Err(DagPaused {
                    step_id,
                    execution_id,
                }
                .into())
            }
            Err(e) => {
                let error_msg = format!("{:#}", e);
                self.collection_repo
                    .update_workflow_execution_status(
                        workflow_exec.id,
                        "failed",
                        None,
                        Some(error_msg.clone()),
                    )
                    .await?;
                broadcast_workflow_event(
                    &self.state,
                    &ctx,
                    workflow_id,
                    WorkflowEventKind::Failed {
                        error: error_msg.clone(),
                    },
                );
                Err(anyhow!("Workflow {} failed: {}", workflow_id, error_msg))
            }
        }
    }
}

/// Topological sort for workflows (adapted from workflow_steps logic).
///
/// Uses Kahn's algorithm to sort workflows by their dependencies.
/// Returns workflow IDs in execution order, or an error if a cycle is detected.
fn topological_sort_workflows(
    workflow_ids: &[Uuid],
    collection_workflows: &[CollectionWorkflowRow],
    edges: &[CollectionWorkflowEdgeRow],
) -> Result<Vec<Uuid>> {
    let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
    let mut adj_list: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    // Initialize
    for &workflow_id in workflow_ids {
        in_degree.insert(workflow_id, 0);
        adj_list.insert(workflow_id, Vec::new());
    }

    // Build adjacency list
    for edge in edges {
        adj_list
            .entry(edge.from_workflow_id)
            .or_default()
            .push(edge.to_workflow_id);
        *in_degree.entry(edge.to_workflow_id).or_insert(0) += 1;
    }

    // Find entry workflows (in_degree == 0)
    let mut queue: VecDeque<Uuid> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    // Sort entry workflows by display_order
    let mut queue_vec: Vec<Uuid> = queue.into_iter().collect();
    queue_vec.sort_by_key(|&id| {
        collection_workflows
            .iter()
            .find(|cw| cw.workflow_id == id)
            .map(|cw| cw.display_order)
            .unwrap_or(0)
    });
    queue = queue_vec.into_iter().collect();

    let mut sorted = Vec::new();

    while let Some(current) = queue.pop_front() {
        sorted.push(current);

        if let Some(neighbors) = adj_list.get(&current) {
            for &next in neighbors {
                if let Some(deg) = in_degree.get_mut(&next) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(next);
                    }
                }
            }
        }
    }

    // Cycle detection
    if sorted.len() != workflow_ids.len() {
        return Err(anyhow!("Cycle detected in workflow collection DAG"));
    }

    Ok(sorted)
}

/// Collect outputs from completed workflows for variable resolution.
///
/// Returns a HashMap of variable_name → JsonValue for all completed workflow outputs.
/// Uses workflow names (not UUIDs) for user-friendly variable references.
/// Stores as nested objects: { "$workflow_analysis": { "result": ... } }
async fn collect_workflow_outputs(
    completed_workflows: &HashMap<Uuid, WorkflowExecutionRow>,
    workflow_repo: &dyn WorkflowRepo,
) -> Result<HashMap<String, JsonValue>> {
    let mut outputs = HashMap::new();

    for (workflow_id, workflow_exec) in completed_workflows {
        // Load workflow to get its name
        let workflow = workflow_repo.get_workflow(*workflow_id).await?;
        let workflow_name = workflow
            .map(|w| w.name.clone())
            .unwrap_or_else(|| workflow_id.to_string());

        if let Some(workflow_outputs) = &workflow_exec.outputs {
            // Store as nested object: $workflow_{name} → { outputs }
            // This allows resolve_path to navigate: {$workflow_analysis.result}
            let workflow_key = format!("$workflow_{}", workflow_name);
            outputs.insert(workflow_key, workflow_outputs.clone());
        }
    }

    Ok(outputs)
}

/// Aggregate step outputs into workflow-level outputs.
///
/// Takes the outputs from all steps in the workflow and combines them into
/// a single JSON object keyed by step output variable names.
fn aggregate_step_outputs(
    step_outputs: &HashMap<String, crate::server::hub::dag::StepOutput>,
) -> JsonValue {
    use serde_json::json;

    let mut aggregated = serde_json::Map::new();

    for output in step_outputs.values() {
        if let Some(structured) = &output.structured_output {
            aggregated.insert(output.variable_name.clone(), structured.clone());
        } else if !output.raw_output.is_empty() {
            aggregated.insert(
                output.variable_name.clone(),
                JsonValue::String(output.raw_output.clone()),
            );
        }
    }

    json!(aggregated)
}

#[cfg(test)]
mod tests;
