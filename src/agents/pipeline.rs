//! Pipeline — chained agent workflows where one agent's output feeds the next.

use serde_json::{self, Value};
use std::collections::HashMap;
use uuid::Uuid;

use super::protocol::AgentId;
use super::cluster::ClusterId;

/// Unique identifier for a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineId(pub Uuid);

impl Default for PipelineId {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// A stage in a pipeline — assigns a specific agent with an optional role.
#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub stage_number: u32,
    pub agent_id: Option<AgentId>,
    pub cluster_id: Option<ClusterId>,
    pub role: Option<String>,
    pub approval_required: bool,
    pub fan_out: bool,
    pub stage_name: String,
    pub input_definitions: serde_json::Value,
    pub output_description: String,
    pub output_schema: serde_json::Value,
}

/// A named pipeline template with ordered stages.
#[derive(Debug, Clone)]
pub struct Pipeline {
    pub id: PipelineId,
    pub name: String,
    pub stages: Vec<PipelineStage>,
}

/// Status of an active pipeline run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineRunStatus {
    Running,
    WaitingForApproval,
    Completed,
    Failed,
}

/// A live execution of a pipeline with a specific task.
#[derive(Debug, Clone)]
pub struct PipelineRun {
    pub id: Uuid,
    pub pipeline_id: PipelineId,
    pub initial_task: String,
    pub current_stage: u32,
    /// Maps stage_number → task_id assigned to that stage's agent.
    pub stage_task_ids: HashMap<u32, Uuid>,
    /// Maps stage_name → structured output JSON from that stage.
    pub stage_outputs: HashMap<String, Value>,
    pub status: PipelineRunStatus,
    /// Number of retries attempted for the current stage.
    pub stage_retries: u32,
    /// Maximum retries per stage before the run is failed.
    pub max_stage_retries: u32,
}

/// Manages pipeline definitions and active runs.
#[derive(Debug, Default)]
pub struct PipelineManager {
    pipelines: HashMap<PipelineId, Pipeline>,
    runs: HashMap<Uuid, PipelineRun>,
    /// Reverse index: task_id → (run_id, stage_number)
    task_to_run: HashMap<Uuid, (Uuid, u32)>,
}

impl PipelineManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new pipeline, returning its ID.
    pub fn create_pipeline(&mut self, name: String) -> PipelineId {
        let id = PipelineId::new();
        let pipeline = Pipeline { id, name, stages: Vec::new() };
        self.pipelines.insert(id, pipeline);
        id
    }

    /// Create a pipeline with a specific ID (for reconstruction from DB).
    pub fn create_pipeline_with_id(&mut self, id: PipelineId, name: String) {
        let pipeline = Pipeline { id, name, stages: Vec::new() };
        self.pipelines.insert(id, pipeline);
    }

    /// Append a stage to a pipeline.
    pub fn add_stage(
        &mut self,
        pipeline_id: PipelineId,
        agent_id: Option<AgentId>,
        cluster_id: Option<ClusterId>,
        role: Option<String>,
        approval_required: bool,
        fan_out: bool,
        stage_name: String,
        input_definitions: Value,
        output_description: String,
        output_schema: Value,
    ) -> Result<u32, PipelineError> {
        let pipeline = self.pipelines.get_mut(&pipeline_id).ok_or(PipelineError::NotFound(pipeline_id))?;
        let stage_number = pipeline.stages.len() as u32;
        pipeline.stages.push(PipelineStage {
            stage_number,
            agent_id,
            cluster_id,
            role,
            approval_required,
            fan_out,
            stage_name,
            input_definitions,
            output_description,
            output_schema,
        });
        Ok(stage_number)
    }

    /// Start a new run of a pipeline. Returns the run_id and first stage.
    /// Caller is responsible for assigning the first stage's task.
    pub fn start_run(&mut self, pipeline_id: PipelineId, task_description: String) -> Result<(Uuid, &PipelineStage), PipelineError> {
        let pipeline = self.pipelines.get(&pipeline_id).ok_or(PipelineError::NotFound(pipeline_id))?;
        if pipeline.stages.is_empty() {
            return Err(PipelineError::NoStages(pipeline_id));
        }
        let run_id = Uuid::new_v4();
        let run = PipelineRun {
            id: run_id,
            pipeline_id,
            initial_task: task_description,
            current_stage: 0,
            stage_task_ids: HashMap::new(),
            stage_outputs: HashMap::new(),
            status: PipelineRunStatus::Running,
            stage_retries: 0,
            max_stage_retries: crate::constants::PIPELINE_MAX_STAGE_RETRIES,
        };
        self.runs.insert(run_id, run);
        let first_stage = &self.pipelines[&pipeline_id].stages[0];
        Ok((run_id, first_stage))
    }

    /// Record the task_id assigned for a specific stage of a run.
    pub fn record_stage_task(&mut self, run_id: Uuid, stage_number: u32, task_id: Uuid) {
        if let Some(run) = self.runs.get_mut(&run_id) {
            run.stage_task_ids.insert(stage_number, task_id);
        }
        self.task_to_run.insert(task_id, (run_id, stage_number));
    }

    /// Look up which pipeline run a task belongs to.
    pub fn lookup_run_by_task(&self, task_id: Uuid) -> Option<(Uuid, u32)> {
        self.task_to_run.get(&task_id).copied()
    }

    /// Advance to the next stage. Returns the next stage, or None if pipeline is complete.
    pub fn advance_stage(&mut self, run_id: Uuid) -> Result<Option<PipelineStage>, PipelineError> {
        let run = self.runs.get_mut(&run_id).ok_or(PipelineError::RunNotFound(run_id))?;
        let pipeline = self.pipelines.get(&run.pipeline_id).ok_or(PipelineError::NotFound(run.pipeline_id))?;

        let next = run.current_stage + 1;
        if next >= pipeline.stages.len() as u32 {
            run.status = PipelineRunStatus::Completed;
            return Ok(None);
        }
        run.current_stage = next;
        run.stage_retries = 0; // Reset retries for new stage
        Ok(Some(pipeline.stages[next as usize].clone()))
    }

    /// Mark a run as waiting for approval.
    pub fn set_waiting_for_approval(&mut self, run_id: Uuid) {
        if let Some(run) = self.runs.get_mut(&run_id) {
            run.status = PipelineRunStatus::WaitingForApproval;
        }
    }

    /// Retry the current stage. Returns the stage to re-execute, or fails the run
    /// if max retries are exceeded.
    pub fn retry_stage(&mut self, run_id: Uuid) -> Result<Option<PipelineStage>, PipelineError> {
        let run = self.runs.get_mut(&run_id).ok_or(PipelineError::RunNotFound(run_id))?;
        if run.stage_retries >= run.max_stage_retries {
            run.status = PipelineRunStatus::Failed;
            return Err(PipelineError::MaxRetriesExceeded(run_id));
        }
        run.stage_retries += 1;
        let pipeline = self.pipelines.get(&run.pipeline_id).ok_or(PipelineError::NotFound(run.pipeline_id))?;
        tracing::info!(
            run_id = %run_id,
            stage = run.current_stage,
            retry = run.stage_retries,
            max_retries = run.max_stage_retries,
            "Retrying pipeline stage"
        );
        Ok(Some(pipeline.stages[run.current_stage as usize].clone()))
    }

    /// Mark a run as failed with a reason.
    pub fn fail_run(&mut self, run_id: Uuid, reason: &str) -> Result<(), PipelineError> {
        let run = self.runs.get_mut(&run_id).ok_or(PipelineError::RunNotFound(run_id))?;
        run.status = PipelineRunStatus::Failed;
        tracing::warn!(
            run_id = %run_id,
            stage = run.current_stage,
            reason = reason,
            "Pipeline run failed"
        );
        Ok(())
    }

    /// Get a pipeline by ID.
    pub fn get_pipeline(&self, id: &PipelineId) -> Option<&Pipeline> {
        self.pipelines.get(id)
    }

    /// Get a run by ID.
    pub fn get_run(&self, run_id: Uuid) -> Option<&PipelineRun> {
        self.runs.get(&run_id)
    }

    /// Get the initial task description for a run.
    pub fn get_run_initial_task(&self, run_id: Uuid) -> Option<&str> {
        self.runs.get(&run_id).map(|r| r.initial_task.as_str())
    }

    /// Record the structured output for a completed stage.
    pub fn record_stage_output(&mut self, run_id: Uuid, stage_name: String, output: Value) {
        if let Some(run) = self.runs.get_mut(&run_id) {
            run.stage_outputs.insert(stage_name, output);
        }
    }

    /// Get all stage outputs for a run (keyed by stage_name).
    pub fn get_stage_outputs(&self, run_id: Uuid) -> Option<&HashMap<String, Value>> {
        self.runs.get(&run_id).map(|r| &r.stage_outputs)
    }

    /// Get the pipeline_id for a run.
    pub fn get_run_pipeline_id(&self, run_id: Uuid) -> Option<PipelineId> {
        self.runs.get(&run_id).map(|r| r.pipeline_id)
    }

    /// Get the stage_name for a given stage_number in a pipeline run.
    pub fn get_stage_name(&self, run_id: Uuid, stage_number: u32) -> Option<String> {
        let run = self.runs.get(&run_id)?;
        let pipeline = self.pipelines.get(&run.pipeline_id)?;
        pipeline.stages.get(stage_number as usize).map(|s| s.stage_name.clone())
    }

    /// List all pipelines.
    pub fn list_pipelines(&self) -> Vec<&Pipeline> {
        self.pipelines.values().collect()
    }
}

/// Parse raw LLM output into structured JSON based on the output schema.
///
/// Tries to extract a JSON object from the output (fenced ```json blocks or bare `{...}`).
/// Falls back to wrapping the raw text as `{"output": "..."}`.
pub fn parse_stage_output(raw: &str, output_schema: &Value) -> Value {
    let has_fields = output_schema.get("fields").and_then(|f| f.as_array()).is_some_and(|a| !a.is_empty());

    if has_fields {
        // Try ```json ... ``` fenced block
        if let Some(start) = raw.find("```json") {
            let after = &raw[start + 7..];
            if let Some(end) = after.find("```") {
                let json_str = after[..end].trim();
                if let Ok(val) = serde_json::from_str::<Value>(json_str) {
                    if val.is_object() {
                        return val;
                    }
                }
            }
        }
        // Try bare JSON object
        if let Some(start) = raw.find('{') {
            if let Some(end) = raw.rfind('}') {
                let json_str = &raw[start..=end];
                if let Ok(val) = serde_json::from_str::<Value>(json_str) {
                    if val.is_object() {
                        return val;
                    }
                }
            }
        }
    }

    serde_json::json!({ "output": raw })
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Pipeline not found: {0:?}")]
    NotFound(PipelineId),
    #[error("Pipeline has no stages: {0:?}")]
    NoStages(PipelineId),
    #[error("Pipeline run not found: {0}")]
    RunNotFound(Uuid),
    #[error("Pipeline run {0} exceeded max stage retries")]
    MaxRetriesExceeded(Uuid),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(n: u128) -> AgentId {
        AgentId(Uuid::from_u128(n))
    }

    fn default_stage_args() -> (String, Value, String, Value) {
        (String::new(), serde_json::json!([]), String::new(), serde_json::json!({"fields": []}))
    }

    macro_rules! add_stage {
        ($mgr:expr, $pid:expr, $agent:expr, $role:expr, $approval:expr) => {{
            let (sn, id, od, os) = default_stage_args();
            $mgr.add_stage($pid, Some($agent), None, $role, $approval, false, sn, id, od, os)
        }};
    }

    #[test]
    fn create_and_list() {
        let mut mgr = PipelineManager::new();
        let id = mgr.create_pipeline("code-review".into());
        let pipelines = mgr.list_pipelines();
        assert_eq!(pipelines.len(), 1);
        assert_eq!(pipelines[0].name, "code-review");
        assert_eq!(pipelines[0].id, id);
    }

    #[test]
    fn add_stages() {
        let mut mgr = PipelineManager::new();
        let pid = mgr.create_pipeline("p".into());
        let s0 = add_stage!(mgr, pid, agent(1), Some("worker".into()), false).unwrap();
        let s1 = add_stage!(mgr, pid, agent(2), Some("reviewer".into()), true).unwrap();
        assert_eq!(s0, 0);
        assert_eq!(s1, 1);
        assert_eq!(mgr.get_pipeline(&pid).unwrap().stages.len(), 2);
    }

    #[test]
    fn add_stage_to_nonexistent_fails() {
        let mut mgr = PipelineManager::new();
        let bad = PipelineId::new();
        assert!(add_stage!(mgr, bad, agent(1), None, false).is_err());
    }

    #[test]
    fn start_run_and_advance() {
        let mut mgr = PipelineManager::new();
        let pid = mgr.create_pipeline("p".into());
        add_stage!(mgr, pid, agent(1), None, false).unwrap();
        add_stage!(mgr, pid, agent(2), None, false).unwrap();
        add_stage!(mgr, pid, agent(3), None, false).unwrap();

        let (run_id, first) = mgr.start_run(pid, "do stuff".into()).unwrap();
        assert_eq!(first.stage_number, 0);
        assert_eq!(first.agent_id, Some(agent(1)));

        // Record first stage task
        let task1 = Uuid::new_v4();
        mgr.record_stage_task(run_id, 0, task1);
        assert_eq!(mgr.lookup_run_by_task(task1), Some((run_id, 0)));

        // Advance to stage 1
        let next = mgr.advance_stage(run_id).unwrap().unwrap();
        assert_eq!(next.stage_number, 1);
        assert_eq!(next.agent_id, Some(agent(2)));

        // Advance to stage 2
        let next = mgr.advance_stage(run_id).unwrap().unwrap();
        assert_eq!(next.stage_number, 2);
        assert_eq!(next.agent_id, Some(agent(3)));

        // Advance past last stage → completed
        let next = mgr.advance_stage(run_id).unwrap();
        assert!(next.is_none());
        assert_eq!(mgr.get_run(run_id).unwrap().status, PipelineRunStatus::Completed);
    }

    #[test]
    fn start_run_no_stages_fails() {
        let mut mgr = PipelineManager::new();
        let pid = mgr.create_pipeline("empty".into());
        assert!(mgr.start_run(pid, "task".into()).is_err());
    }

    #[test]
    fn fail_run() {
        let mut mgr = PipelineManager::new();
        let pid = mgr.create_pipeline("p".into());
        add_stage!(mgr, pid, agent(1), None, false).unwrap();
        let (run_id, _) = mgr.start_run(pid, "task".into()).unwrap();
        mgr.fail_run(run_id, "test failure").unwrap();
        assert_eq!(mgr.get_run(run_id).unwrap().status, PipelineRunStatus::Failed);
    }

    #[test]
    fn create_pipeline_with_id() {
        let mut mgr = PipelineManager::new();
        let id = PipelineId::new();
        mgr.create_pipeline_with_id(id, "restored".into());
        assert_eq!(mgr.get_pipeline(&id).unwrap().name, "restored");
    }

    #[test]
    fn record_and_get_stage_outputs() {
        let mut mgr = PipelineManager::new();
        let pid = mgr.create_pipeline("p".into());
        add_stage!(mgr, pid, agent(1), None, false).unwrap();
        let (run_id, _) = mgr.start_run(pid, "task".into()).unwrap();

        mgr.record_stage_output(run_id, "analysis".into(), serde_json::json!({"score": 85}));
        let outputs = mgr.get_stage_outputs(run_id).unwrap();
        assert_eq!(outputs["analysis"]["score"], 85);
    }

    #[test]
    fn parse_stage_output_json_fence() {
        let schema = serde_json::json!({"fields": [{"name": "plan"}]});
        let raw = "Here is my analysis:\n```json\n{\"plan\": \"do stuff\"}\n```\nDone.";
        let result = super::parse_stage_output(raw, &schema);
        assert_eq!(result["plan"], "do stuff");
    }

    #[test]
    fn parse_stage_output_bare_json() {
        let schema = serde_json::json!({"fields": [{"name": "x"}]});
        let raw = "Result: {\"x\": 42}";
        let result = super::parse_stage_output(raw, &schema);
        assert_eq!(result["x"], 42);
    }

    #[test]
    fn parse_stage_output_no_schema() {
        let schema = serde_json::json!({"fields": []});
        let raw = "just plain text";
        let result = super::parse_stage_output(raw, &schema);
        assert_eq!(result["output"], "just plain text");
    }

    #[test]
    fn retry_stage_increments_and_returns_stage() {
        let mut mgr = PipelineManager::new();
        let pid = mgr.create_pipeline("p".into());
        add_stage!(mgr, pid, agent(1), None, false).unwrap();
        add_stage!(mgr, pid, agent(2), None, false).unwrap();
        let (run_id, _) = mgr.start_run(pid, "task".into()).unwrap();

        // First retry should succeed
        let stage = mgr.retry_stage(run_id).unwrap().unwrap();
        assert_eq!(stage.stage_number, 0);
        assert_eq!(mgr.get_run(run_id).unwrap().stage_retries, 1);
    }

    #[test]
    fn retry_stage_max_retries_fails_run() {
        let mut mgr = PipelineManager::new();
        let pid = mgr.create_pipeline("p".into());
        add_stage!(mgr, pid, agent(1), None, false).unwrap();
        let (run_id, _) = mgr.start_run(pid, "task".into()).unwrap();

        // First retry succeeds (max_stage_retries = 1)
        mgr.retry_stage(run_id).unwrap();

        // Second retry exceeds max
        let result = mgr.retry_stage(run_id);
        assert!(matches!(result, Err(PipelineError::MaxRetriesExceeded(_))));
        assert_eq!(mgr.get_run(run_id).unwrap().status, PipelineRunStatus::Failed);
    }

    #[test]
    fn advance_stage_resets_retries() {
        let mut mgr = PipelineManager::new();
        let pid = mgr.create_pipeline("p".into());
        add_stage!(mgr, pid, agent(1), None, false).unwrap();
        add_stage!(mgr, pid, agent(2), None, false).unwrap();
        let (run_id, _) = mgr.start_run(pid, "task".into()).unwrap();

        // Retry stage 0 once
        mgr.retry_stage(run_id).unwrap();
        assert_eq!(mgr.get_run(run_id).unwrap().stage_retries, 1);

        // Advance to stage 1 — retries should reset
        mgr.advance_stage(run_id).unwrap();
        assert_eq!(mgr.get_run(run_id).unwrap().stage_retries, 0);
    }

    #[test]
    fn fail_run_nonexistent_returns_error() {
        let mut mgr = PipelineManager::new();
        let result = mgr.fail_run(Uuid::new_v4(), "no such run");
        assert!(matches!(result, Err(PipelineError::RunNotFound(_))));
    }
}
