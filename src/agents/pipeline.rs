//! Pipeline — chained agent workflows where one agent's output feeds the next.

use std::collections::HashMap;
use uuid::Uuid;

use super::agent::AgentId;

/// Unique identifier for a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineId(pub Uuid);

impl PipelineId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// A stage in a pipeline — assigns a specific agent with an optional role.
#[derive(Debug, Clone)]
pub struct PipelineStage {
    pub stage_number: u32,
    pub agent_id: AgentId,
    pub role: Option<String>,
    pub approval_required: bool,
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
    pub status: PipelineRunStatus,
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
        let pipeline = Pipeline {
            id,
            name,
            stages: Vec::new(),
        };
        self.pipelines.insert(id, pipeline);
        id
    }

    /// Create a pipeline with a specific ID (for reconstruction from DB).
    pub fn create_pipeline_with_id(&mut self, id: PipelineId, name: String) {
        let pipeline = Pipeline {
            id,
            name,
            stages: Vec::new(),
        };
        self.pipelines.insert(id, pipeline);
    }

    /// Append a stage to a pipeline.
    pub fn add_stage(
        &mut self,
        pipeline_id: PipelineId,
        agent_id: AgentId,
        role: Option<String>,
        approval_required: bool,
    ) -> Result<u32, PipelineError> {
        let pipeline = self
            .pipelines
            .get_mut(&pipeline_id)
            .ok_or(PipelineError::NotFound(pipeline_id))?;
        let stage_number = pipeline.stages.len() as u32;
        pipeline.stages.push(PipelineStage {
            stage_number,
            agent_id,
            role,
            approval_required,
        });
        Ok(stage_number)
    }

    /// Start a new run of a pipeline. Returns the run_id and first stage.
    /// Caller is responsible for assigning the first stage's task.
    pub fn start_run(
        &mut self,
        pipeline_id: PipelineId,
        task_description: String,
    ) -> Result<(Uuid, &PipelineStage), PipelineError> {
        let pipeline = self
            .pipelines
            .get(&pipeline_id)
            .ok_or(PipelineError::NotFound(pipeline_id))?;
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
            status: PipelineRunStatus::Running,
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
        let run = self
            .runs
            .get_mut(&run_id)
            .ok_or(PipelineError::RunNotFound(run_id))?;
        let pipeline = self
            .pipelines
            .get(&run.pipeline_id)
            .ok_or(PipelineError::NotFound(run.pipeline_id))?;

        let next = run.current_stage + 1;
        if next >= pipeline.stages.len() as u32 {
            run.status = PipelineRunStatus::Completed;
            return Ok(None);
        }
        run.current_stage = next;
        Ok(Some(pipeline.stages[next as usize].clone()))
    }

    /// Mark a run as waiting for approval.
    pub fn set_waiting_for_approval(&mut self, run_id: Uuid) {
        if let Some(run) = self.runs.get_mut(&run_id) {
            run.status = PipelineRunStatus::WaitingForApproval;
        }
    }

    /// Mark a run as failed.
    pub fn fail_run(&mut self, run_id: Uuid) {
        if let Some(run) = self.runs.get_mut(&run_id) {
            run.status = PipelineRunStatus::Failed;
        }
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

    /// List all pipelines.
    pub fn list_pipelines(&self) -> Vec<&Pipeline> {
        self.pipelines.values().collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Pipeline not found: {0:?}")]
    NotFound(PipelineId),
    #[error("Pipeline has no stages: {0:?}")]
    NoStages(PipelineId),
    #[error("Pipeline run not found: {0}")]
    RunNotFound(Uuid),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(n: u128) -> AgentId {
        AgentId(Uuid::from_u128(n))
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
        let s0 = mgr.add_stage(pid, agent(1), Some("worker".into()), false).unwrap();
        let s1 = mgr.add_stage(pid, agent(2), Some("reviewer".into()), true).unwrap();
        assert_eq!(s0, 0);
        assert_eq!(s1, 1);
        assert_eq!(mgr.get_pipeline(&pid).unwrap().stages.len(), 2);
    }

    #[test]
    fn add_stage_to_nonexistent_fails() {
        let mut mgr = PipelineManager::new();
        let bad = PipelineId::new();
        assert!(mgr.add_stage(bad, agent(1), None, false).is_err());
    }

    #[test]
    fn start_run_and_advance() {
        let mut mgr = PipelineManager::new();
        let pid = mgr.create_pipeline("p".into());
        mgr.add_stage(pid, agent(1), None, false).unwrap();
        mgr.add_stage(pid, agent(2), None, false).unwrap();
        mgr.add_stage(pid, agent(3), None, false).unwrap();

        let (run_id, first) = mgr.start_run(pid, "do stuff".into()).unwrap();
        assert_eq!(first.stage_number, 0);
        assert_eq!(first.agent_id, agent(1));

        // Record first stage task
        let task1 = Uuid::new_v4();
        mgr.record_stage_task(run_id, 0, task1);
        assert_eq!(mgr.lookup_run_by_task(task1), Some((run_id, 0)));

        // Advance to stage 1
        let next = mgr.advance_stage(run_id).unwrap().unwrap();
        assert_eq!(next.stage_number, 1);
        assert_eq!(next.agent_id, agent(2));

        // Advance to stage 2
        let next = mgr.advance_stage(run_id).unwrap().unwrap();
        assert_eq!(next.stage_number, 2);
        assert_eq!(next.agent_id, agent(3));

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
        mgr.add_stage(pid, agent(1), None, false).unwrap();
        let (run_id, _) = mgr.start_run(pid, "task".into()).unwrap();
        mgr.fail_run(run_id);
        assert_eq!(mgr.get_run(run_id).unwrap().status, PipelineRunStatus::Failed);
    }

    #[test]
    fn create_pipeline_with_id() {
        let mut mgr = PipelineManager::new();
        let id = PipelineId::new();
        mgr.create_pipeline_with_id(id, "restored".into());
        assert_eq!(mgr.get_pipeline(&id).unwrap().name, "restored");
    }
}
