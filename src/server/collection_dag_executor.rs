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
    async fn execute_collection_sequential(
        &self,
        run_id: Uuid,
        collection_workflows: &[CollectionWorkflowRow],
        edges: &[CollectionWorkflowEdgeRow],
        user_id: Uuid,
    ) -> Result<()> {
        // Topological sort workflows
        let workflow_ids: Vec<Uuid> = collection_workflows.iter().map(|cw| cw.workflow_id).collect();
        let sorted_workflow_ids = topological_sort_workflows(&workflow_ids, collection_workflows, edges)?;

        let mut completed_workflows: HashMap<Uuid, WorkflowExecutionRow> = HashMap::new();

        for workflow_id in sorted_workflow_ids {
            // Collect outputs from completed workflows for variable resolution
            let prior_outputs = collect_workflow_outputs(&completed_workflows);

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
    /// TODO: Implement true parallel execution with tokio::spawn.
    /// For now, this uses sequential execution to avoid Send/Sync complexity.
    async fn execute_collection_parallel(
        &self,
        run_id: Uuid,
        collection_workflows: &[CollectionWorkflowRow],
        edges: &[CollectionWorkflowEdgeRow],
        user_id: Uuid,
    ) -> Result<()> {
        // TODO: Implement true parallel execution
        // For now, just use sequential execution
        self.execute_collection_sequential(run_id, collection_workflows, edges, user_id).await
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
        use crate::llm::{AnthropicClient, RateLimitedProvider, RetryingProvider};

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

        // 4. Create LLM provider
        let provider: Arc<dyn LLMProvider> = match AnthropicClient::from_env() {
            Ok(p) => Arc::new(RetryingProvider::with_defaults(RateLimitedProvider::with_defaults(p))),
            Err(e) => {
                let error_msg = format!("Failed to initialize LLM provider: {}", e);
                self.collection_repo
                    .update_workflow_execution_status(workflow_exec.id, "failed", None, Some(error_msg.clone()))
                    .await?;
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
                    .update_workflow_execution_status(
                        workflow_exec.id,
                        "completed",
                        Some(workflow_outputs.clone()),
                        None,
                    )
                    .await?;

                // Store execution variables for cross-workflow variable resolution
                store_workflow_variables(
                    &*self.collection_repo,
                    collection_run_id,
                    workflow_exec.id,
                    workflow_id,
                    &workflow_outputs,
                )
                .await?;

                Ok(workflow_exec)
            }
            Err(e) => {
                let error_msg = format!("{:#}", e);
                let _workflow_exec = self
                    .collection_repo
                    .update_workflow_execution_status(workflow_exec.id, "failed", None, Some(error_msg.clone()))
                    .await?;
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
        adj_list.entry(edge.from_workflow_id).or_insert_with(Vec::new).push(edge.to_workflow_id);
        *in_degree.entry(edge.to_workflow_id).or_insert(0) += 1;
    }

    // Find entry workflows (in_degree == 0)
    let mut queue: VecDeque<Uuid> = in_degree.iter().filter(|(_, &deg)| deg == 0).map(|(&id, _)| id).collect();

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
fn collect_workflow_outputs(completed_workflows: &HashMap<Uuid, WorkflowExecutionRow>) -> HashMap<String, JsonValue> {
    let mut outputs = HashMap::new();

    for (workflow_id, workflow_exec) in completed_workflows {
        if let Some(workflow_outputs) = &workflow_exec.outputs {
            // Store each output with a workflow-scoped key: $workflow_{id}.{key}
            if let Some(obj) = workflow_outputs.as_object() {
                for (key, value) in obj {
                    let scoped_key = format!("$workflow_{}.{}", workflow_id, key);
                    outputs.insert(scoped_key, value.clone());
                }
            }
        }
    }

    outputs
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
async fn store_workflow_variables<R>(
    collection_repo: &R,
    collection_run_id: Uuid,
    workflow_execution_id: Uuid,
    workflow_id: Uuid,
    workflow_outputs: &JsonValue,
) -> Result<()>
where
    R: WorkflowCollectionRepo + Send + Sync,
{
    if let Some(obj) = workflow_outputs.as_object() {
        for (key, value) in obj {
            let variable_path = format!("$workflow_{}.{}", workflow_id, key);

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
