//! Collection DAG executor for multi-tier workflow orchestration.
//!
//! This module implements the execution of workflow collections, which are DAGs of workflows.
//! Collections can execute workflows sequentially or in parallel (respecting dependencies).

use anyhow::{anyhow, Result};
use futures::future;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::traits::{WorkflowCollectionRepo, WorkflowRepo};
use crate::db::{CollectionRunRow, CollectionWorkflowEdgeRow, CollectionWorkflowRow, WorkflowExecutionRow};
use crate::llm::LLMProvider;
use crate::server::dag_executor::{execute_workflow, WorkflowExecutionContext};
use crate::server::state::AppState;

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
    pub fn new(collection_repo: Arc<R>, workflow_repo: Arc<dyn WorkflowRepo>, state: Arc<AppState>) -> Self {
        Self {
            collection_repo,
            workflow_repo,
            state,
        }
    }

    /// Execute a workflow collection (DAG of workflows).
    pub async fn execute_collection(&self, collection_id: Uuid, user_id: Uuid) -> Result<CollectionRunRow> {
        // 1. Load collection
        let collection = self
            .collection_repo
            .get_collection(collection_id)
            .await?
            .ok_or_else(|| anyhow!("Collection not found: {}", collection_id))?;

        // 2. Load workflows and edges
        let collection_workflows = self.collection_repo.list_collection_workflows(collection_id).await?;
        let edges = self.collection_repo.list_collection_edges(collection_id).await?;

        // 3. Create collection_run row
        let run = self.collection_repo.create_collection_run(collection_id, user_id).await?;

        // 4. Execute based on execution_mode
        let result = match collection.execution_mode.as_str() {
            "sequential" => self.execute_collection_sequential(run.id, &collection_workflows, &edges, user_id).await,
            "parallel" => self.execute_collection_parallel(run.id, &collection_workflows, &edges, user_id).await,
            _ => Err(anyhow!("Unknown execution mode: {}", collection.execution_mode)),
        };

        // 5. Update collection_run status
        let final_run = match result {
            Ok(_) => self.collection_repo.update_collection_run_status(run.id, "completed", None).await?,
            Err(e) => {
                let error_msg = format!("{:#}", e);
                self.collection_repo.update_collection_run_status(run.id, "failed", Some(error_msg)).await?
            }
        };

        Ok(final_run)
    }

    /// Execute workflows sequentially (one-at-a-time).
    async fn execute_collection_sequential(&self, run_id: Uuid, collection_workflows: &[CollectionWorkflowRow], edges: &[CollectionWorkflowEdgeRow], user_id: Uuid) -> Result<()> {
        // Topological sort workflows
        let workflow_ids: Vec<Uuid> = collection_workflows.iter().map(|cw| cw.workflow_id).collect();
        let sorted_workflow_ids = topological_sort_workflows(&workflow_ids, collection_workflows, edges)?;

        let mut completed_workflows: HashMap<Uuid, WorkflowExecutionRow> = HashMap::new();

        for workflow_id in sorted_workflow_ids {
            // Collect outputs from completed workflows for variable resolution
            let prior_outputs = collect_workflow_outputs(&completed_workflows, &*self.workflow_repo).await?;

            // Execute workflow and capture outputs
            let workflow_exec = self.execute_workflow_in_collection(run_id, workflow_id, user_id, &prior_outputs).await?;

            completed_workflows.insert(workflow_id, workflow_exec);
        }

        Ok(())
    }

    /// Execute workflows in parallel (respecting DAG dependencies).
    ///
    /// Spawns multiple workflow executions concurrently, respecting the DAG structure.
    /// Entry workflows (no dependencies) start immediately. Each workflow, upon completion,
    /// triggers its dependent workflows if all their dependencies are satisfied.
    async fn execute_collection_parallel(&self, run_id: Uuid, collection_workflows: &[CollectionWorkflowRow], edges: &[CollectionWorkflowEdgeRow], user_id: Uuid) -> Result<()> {
        use tokio::sync::RwLock;

        // Build dependency graph
        let workflow_ids: Vec<Uuid> = collection_workflows.iter().map(|cw| cw.workflow_id).collect();
        let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
        let mut children: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

        for &workflow_id in &workflow_ids {
            in_degree.insert(workflow_id, 0);
            children.insert(workflow_id, Vec::new());
        }

        for edge in edges {
            children.entry(edge.from_workflow_id).or_default().push(edge.to_workflow_id);
            *in_degree.entry(edge.to_workflow_id).or_default() += 1;
        }

        // Shared state for parallel execution
        let completed = Arc::new(RwLock::new(HashMap::<Uuid, WorkflowExecutionRow>::new()));
        let in_degree = Arc::new(RwLock::new(in_degree));
        let children = Arc::new(children);

        // Find entry workflows (no dependencies)
        let ready: Vec<Uuid> = {
            let deg = in_degree.read().await;
            deg.iter().filter(|(_, &d)| d == 0).map(|(&id, _)| id).collect()
        };

        // Channel to collect errors from spawned tasks
        let (error_tx, mut error_rx) = tokio::sync::mpsc::channel::<anyhow::Error>(10);

        // Spawn entry workflows
        let mut handles = Vec::new();
        for workflow_id in ready {
            let handle = self.spawn_workflow_with_cascade(run_id, workflow_id, user_id, Arc::clone(&completed), Arc::clone(&in_degree), Arc::clone(&children), error_tx.clone());
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
            return Err(anyhow::anyhow!("Parallel execution failed with {} error(s): {}", errors.len(), errors.first().unwrap()));
        }

        Ok(())
    }

    /// Spawn a workflow execution task that cascades to dependent workflows upon completion.
    fn spawn_workflow_with_cascade(
        &self,
        run_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
        completed: Arc<tokio::sync::RwLock<HashMap<Uuid, WorkflowExecutionRow>>>,
        in_degree: Arc<tokio::sync::RwLock<HashMap<Uuid, usize>>>,
        children: Arc<HashMap<Uuid, Vec<Uuid>>>,
        error_tx: tokio::sync::mpsc::Sender<anyhow::Error>,
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

            let workflow_exec = match executor.execute_workflow_in_collection(run_id, workflow_id, user_id, &prior_outputs).await {
                Ok(exec) => exec,
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
                let handle = executor.spawn_workflow_with_cascade(run_id, child_id, user_id, Arc::clone(&completed), Arc::clone(&in_degree), Arc::clone(&children), error_tx.clone());
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
    async fn execute_workflow_in_collection(&self, collection_run_id: Uuid, workflow_id: Uuid, user_id: Uuid, prior_workflow_outputs: &HashMap<String, JsonValue>) -> Result<WorkflowExecutionRow> {
        use crate::llm::{AnthropicClient, RateLimitedProvider, RetryingProvider};

        // 1. Create workflow_execution record
        let workflow_exec = self.collection_repo.create_workflow_execution(collection_run_id, workflow_id, user_id).await?;

        // 2. Update status to "running"
        let workflow_exec = self.collection_repo.update_workflow_execution_status(workflow_exec.id, "running", None, None).await?;

        // 3. Load workflow steps and edges
        let steps = self.workflow_repo.list_steps(workflow_id).await?;
        let edges = self.workflow_repo.list_edges(workflow_id).await?;

        // 4. Create LLM provider
        let provider: Arc<dyn LLMProvider> = match AnthropicClient::from_env() {
            Ok(p) => Arc::new(RetryingProvider::with_defaults(RateLimitedProvider::with_defaults(p))),
            Err(e) => {
                let error_msg = format!("Failed to initialize LLM provider: {}", e);
                self.collection_repo.update_workflow_execution_status(workflow_exec.id, "failed", None, Some(error_msg.clone())).await?;
                return Err(anyhow!(error_msg));
            }
        };

        // 5. Create execution context
        let ctx = WorkflowExecutionContext {
            stage_execution_id: workflow_exec.id, // Reuse workflow_execution_id as stage_execution_id
            run_id: collection_run_id,
            user_id,
            initial_input: String::new(), // TODO: support initial input from collection
            prior_outputs: prior_workflow_outputs.clone(),
            execution_context: None, // TODO: support tool execution context
        };

        // 6. Execute workflow DAG
        let result = execute_workflow(&self.state, provider, ctx, steps, edges).await;

        // 7. Handle result and update workflow_execution
        match result {
            Ok(wf_result) => {
                // Aggregate step outputs into workflow-level outputs
                let workflow_outputs = aggregate_step_outputs(&wf_result.outputs);

                // Update workflow_execution with outputs
                let workflow_exec = self
                    .collection_repo
                    .update_workflow_execution_status(workflow_exec.id, "completed", Some(workflow_outputs.clone()), None)
                    .await?;

                // Store execution variables for cross-workflow variable resolution
                store_workflow_variables(&*self.collection_repo, &*self.workflow_repo, collection_run_id, workflow_exec.id, workflow_id, &workflow_outputs).await?;

                Ok(workflow_exec)
            }
            Err(e) => {
                let error_msg = format!("{:#}", e);
                let _workflow_exec = self.collection_repo.update_workflow_execution_status(workflow_exec.id, "failed", None, Some(error_msg.clone())).await?;
                Err(anyhow!("Workflow {} failed: {}", workflow_id, error_msg))
            }
        }
    }
}

/// Topological sort for workflows (adapted from workflow_steps logic).
///
/// Uses Kahn's algorithm to sort workflows by their dependencies.
/// Returns workflow IDs in execution order, or an error if a cycle is detected.
fn topological_sort_workflows(workflow_ids: &[Uuid], collection_workflows: &[CollectionWorkflowRow], edges: &[CollectionWorkflowEdgeRow]) -> Result<Vec<Uuid>> {
    let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
    let mut adj_list: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    // Initialize
    for &workflow_id in workflow_ids {
        in_degree.insert(workflow_id, 0);
        adj_list.insert(workflow_id, Vec::new());
    }

    // Build adjacency list
    for edge in edges {
        adj_list.entry(edge.from_workflow_id).or_insert_with(Vec::new).push(edge.to_workflow_id);
        *in_degree.entry(edge.to_workflow_id).or_insert(0) += 1;
    }

    // Find entry workflows (in_degree == 0)
    let mut queue: VecDeque<Uuid> = in_degree.iter().filter(|(_, &deg)| deg == 0).map(|(&id, _)| id).collect();

    // Sort entry workflows by display_order
    let mut queue_vec: Vec<Uuid> = queue.into_iter().collect();
    queue_vec.sort_by_key(|&id| collection_workflows.iter().find(|cw| cw.workflow_id == id).map(|cw| cw.display_order).unwrap_or(0));
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
async fn collect_workflow_outputs(completed_workflows: &HashMap<Uuid, WorkflowExecutionRow>, workflow_repo: &dyn WorkflowRepo) -> Result<HashMap<String, JsonValue>> {
    let mut outputs = HashMap::new();

    for (workflow_id, workflow_exec) in completed_workflows {
        // Load workflow to get its name
        let workflow = workflow_repo.get_workflow(*workflow_id).await?;
        let workflow_name = workflow.map(|w| w.name.clone()).unwrap_or_else(|| workflow_id.to_string());

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
fn aggregate_step_outputs(step_outputs: &HashMap<String, crate::server::dag_executor::StepOutput>) -> JsonValue {
    use serde_json::json;

    let mut aggregated = serde_json::Map::new();

    for (_step_id, output) in step_outputs {
        if let Some(structured) = &output.structured_output {
            // Use the output variable name as the key
            aggregated.insert(output.variable_name.clone(), structured.clone());
        }
    }

    json!(aggregated)
}

/// Store workflow execution variables for cross-workflow variable resolution.
///
/// Creates execution_variables rows for each output variable from the workflow.
/// Uses workflow names (not UUIDs) for user-friendly variable paths.
async fn store_workflow_variables<R>(
    collection_repo: &R,
    workflow_repo: &dyn WorkflowRepo,
    collection_run_id: Uuid,
    workflow_execution_id: Uuid,
    workflow_id: Uuid,
    workflow_outputs: &JsonValue,
) -> Result<()>
where
    R: WorkflowCollectionRepo + Send + Sync,
{
    // Load workflow to get its name
    let workflow = workflow_repo.get_workflow(workflow_id).await?;
    let workflow_name = workflow.map(|w| w.name.clone()).unwrap_or_else(|| workflow_id.to_string());

    if let Some(obj) = workflow_outputs.as_object() {
        for (key, value) in obj {
            let variable_path = format!("$workflow_{}.{}", workflow_name, key);

            collection_repo
                .create_execution_variable(
                    Some(collection_run_id),
                    Some(workflow_execution_id),
                    None, // step_execution_id
                    key,
                    &variable_path,
                    value.clone(),
                )
                .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort_simple() {
        // A -> B -> C
        let workflow_ids = vec![
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
        ];

        let collection_workflows = vec![
            CollectionWorkflowRow {
                collection_id: Uuid::new_v4(),
                workflow_id: workflow_ids[0],
                display_order: 0,
                execution_mode: None,
            },
            CollectionWorkflowRow {
                collection_id: Uuid::new_v4(),
                workflow_id: workflow_ids[1],
                display_order: 1,
                execution_mode: None,
            },
            CollectionWorkflowRow {
                collection_id: Uuid::new_v4(),
                workflow_id: workflow_ids[2],
                display_order: 2,
                execution_mode: None,
            },
        ];

        let edges = vec![
            CollectionWorkflowEdgeRow {
                from_workflow_id: workflow_ids[0],
                to_workflow_id: workflow_ids[1],
                collection_id: Uuid::new_v4(),
            },
            CollectionWorkflowEdgeRow {
                from_workflow_id: workflow_ids[1],
                to_workflow_id: workflow_ids[2],
                collection_id: Uuid::new_v4(),
            },
        ];

        let sorted = topological_sort_workflows(&workflow_ids, &collection_workflows, &edges).unwrap();
        assert_eq!(sorted, workflow_ids);
    }

    #[test]
    fn test_topological_sort_diamond() {
        // A -> B -> D
        // A -> C -> D
        let a = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let c = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
        let d = Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap();

        let workflow_ids = vec![a, b, c, d];

        let collection_workflows = vec![
            CollectionWorkflowRow {
                collection_id: Uuid::new_v4(),
                workflow_id: a,
                display_order: 0,
                execution_mode: None,
            },
            CollectionWorkflowRow {
                collection_id: Uuid::new_v4(),
                workflow_id: b,
                display_order: 1,
                execution_mode: None,
            },
            CollectionWorkflowRow {
                collection_id: Uuid::new_v4(),
                workflow_id: c,
                display_order: 2,
                execution_mode: None,
            },
            CollectionWorkflowRow {
                collection_id: Uuid::new_v4(),
                workflow_id: d,
                display_order: 3,
                execution_mode: None,
            },
        ];

        let edges = vec![
            CollectionWorkflowEdgeRow {
                from_workflow_id: a,
                to_workflow_id: b,
                collection_id: Uuid::new_v4(),
            },
            CollectionWorkflowEdgeRow {
                from_workflow_id: a,
                to_workflow_id: c,
                collection_id: Uuid::new_v4(),
            },
            CollectionWorkflowEdgeRow {
                from_workflow_id: b,
                to_workflow_id: d,
                collection_id: Uuid::new_v4(),
            },
            CollectionWorkflowEdgeRow {
                from_workflow_id: c,
                to_workflow_id: d,
                collection_id: Uuid::new_v4(),
            },
        ];

        let sorted = topological_sort_workflows(&workflow_ids, &collection_workflows, &edges).unwrap();

        // A must come first, D must come last
        assert_eq!(sorted[0], a);
        assert_eq!(sorted[3], d);
        // B and C can be in any order
        assert!(sorted.contains(&b));
        assert!(sorted.contains(&c));
    }

    #[tokio::test]
    async fn test_variable_resolution() {
        use crate::db::{WorkflowExecutionRow, WorkflowRow};
        use chrono::Utc;

        // Mock workflow repo
        struct MockWorkflowRepo;

        #[async_trait::async_trait]
        impl crate::db::traits::WorkflowRepo for MockWorkflowRepo {
            async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowRow>> {
                Ok(Some(WorkflowRow {
                    id,
                    user_id: Uuid::new_v4(),
                    name: "test_workflow".to_string(),
                    description: String::new(),
                    execution_mode: "parallel".to_string(),
                    created_at: Utc::now(),
                    version: 1,
                }))
            }

            // Stub other methods (not used in test)
            async fn create_workflow(&self, _: Uuid, _: String, _: String) -> Result<WorkflowRow> {
                unimplemented!()
            }
            async fn list_workflows(&self, _: Uuid) -> Result<Vec<WorkflowRow>> {
                unimplemented!()
            }
            async fn update_workflow(&self, _: Uuid, _: Option<String>, _: Option<String>) -> Result<WorkflowRow> {
                unimplemented!()
            }
            async fn delete_workflow(&self, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            async fn create_step(&self, _: crate::db::WorkflowStepRow) -> Result<crate::db::WorkflowStepRow> {
                unimplemented!()
            }
            async fn get_step(&self, _: Uuid) -> Result<Option<crate::db::WorkflowStepRow>> {
                unimplemented!()
            }
            async fn list_steps(&self, _: Uuid) -> Result<Vec<crate::db::WorkflowStepRow>> {
                unimplemented!()
            }
            async fn update_step(&self, _: crate::db::WorkflowStepRow) -> Result<crate::db::WorkflowStepRow> {
                unimplemented!()
            }
            async fn delete_step(&self, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            async fn set_edges(&self, _: Uuid, _: Vec<crate::db::WorkflowStepEdgeRow>) -> Result<()> {
                unimplemented!()
            }
            async fn list_edges(&self, _: Uuid) -> Result<Vec<crate::db::WorkflowStepEdgeRow>> {
                unimplemented!()
            }
            async fn add_edge(&self, _: Uuid, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            async fn remove_edge(&self, _: Uuid, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            async fn list_step_documents(&self, _: Uuid) -> Result<Vec<crate::db::StepDocumentRow>> {
                unimplemented!()
            }
            async fn add_step_document(&self, _: Uuid, _: Uuid) -> Result<()> {
                unimplemented!()
            }
            async fn remove_step_document(&self, _: Uuid, _: Uuid) -> Result<()> {
                unimplemented!()
            }
        }

        // Create test data
        let workflow_id = Uuid::new_v4();
        let mut completed_workflows = HashMap::new();

        let workflow_exec = WorkflowExecutionRow {
            id: Uuid::new_v4(),
            collection_run_id: Uuid::new_v4(),
            workflow_id,
            user_id: Uuid::new_v4(),
            status: "completed".to_string(),
            started_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
            outputs: Some(serde_json::json!({
                "analysis": {"findings": ["issue1", "issue2"]},
                "summary": "Test complete"
            })),
            error: None,
        };

        completed_workflows.insert(workflow_id, workflow_exec);

        // Test variable collection
        let repo = MockWorkflowRepo;
        let outputs = collect_workflow_outputs(&completed_workflows, &repo).await.unwrap();

        // Verify structure: $workflow_test_workflow -> { analysis: ..., summary: ... }
        assert!(outputs.contains_key("$workflow_test_workflow"));

        let workflow_outputs = outputs.get("$workflow_test_workflow").unwrap();
        assert!(workflow_outputs.get("analysis").is_some());
        assert!(workflow_outputs.get("summary").is_some());

        // Verify it would work with resolve_variables
        let template = "Found: {$workflow_test_workflow.analysis.findings.0}";
        let resolved = crate::server::dag_executor::resolve_variables(template, &HashMap::new(), &outputs);
        // Note: JSON strings are resolved with quotes (this is correct behavior)
        assert_eq!(resolved, "Found: issue1");
    }

    #[test]
    fn test_topological_sort_cycle() {
        // A -> B -> C -> A (cycle!)
        let workflow_ids = vec![
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
        ];

        let collection_workflows = vec![
            CollectionWorkflowRow {
                collection_id: Uuid::new_v4(),
                workflow_id: workflow_ids[0],
                display_order: 0,
                execution_mode: None,
            },
            CollectionWorkflowRow {
                collection_id: Uuid::new_v4(),
                workflow_id: workflow_ids[1],
                display_order: 1,
                execution_mode: None,
            },
            CollectionWorkflowRow {
                collection_id: Uuid::new_v4(),
                workflow_id: workflow_ids[2],
                display_order: 2,
                execution_mode: None,
            },
        ];

        let edges = vec![
            CollectionWorkflowEdgeRow {
                from_workflow_id: workflow_ids[0],
                to_workflow_id: workflow_ids[1],
                collection_id: Uuid::new_v4(),
            },
            CollectionWorkflowEdgeRow {
                from_workflow_id: workflow_ids[1],
                to_workflow_id: workflow_ids[2],
                collection_id: Uuid::new_v4(),
            },
            CollectionWorkflowEdgeRow {
                from_workflow_id: workflow_ids[2],
                to_workflow_id: workflow_ids[0],
                collection_id: Uuid::new_v4(),
            },
        ];

        let result = topological_sort_workflows(&workflow_ids, &collection_workflows, &edges);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cycle detected"));
    }
}
