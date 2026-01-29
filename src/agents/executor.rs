//! Agent task execution loop

use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use super::agent::{Agent, AgentError};
use super::channels::{
    AgentCommand, AgentResponse, ApprovalRequest, ContextRequest, ContextResponse, ProgressUpdate,
    RoleContext, TaskAssignment, TaskResult,
};
use super::roles::{CommunicationStyle, OutputFormat};
use crate::llm::{LLMRequest, LLMResponse, Message};
use crate::types::{AgentStatus, TaskStatus};

impl Agent {
    /// Run the agent's main execution loop
    ///
    /// This is the primary entry point for agent execution. It should be
    /// spawned as a tokio task.
    #[instrument(skip(self), fields(agent_id = ?self.id, tier = ?self.tier))]
    pub async fn run(mut self) -> Result<(), AgentError> {
        info!("Agent starting run loop");

        loop {
            // Wait for next command
            let command = match self.recv_command().await {
                Some(cmd) => cmd,
                None => {
                    warn!("Command channel closed, shutting down");
                    break;
                }
            };

            // Process the command
            match command {
                AgentCommand::Shutdown => {
                    info!("Received shutdown command");
                    self.handle_shutdown().await?;
                    break;
                }

                AgentCommand::AssignTask(assignment) => {
                    info!(task_id = ?assignment.task_id, "Received task assignment");
                    if let Err(e) = self.handle_task_assignment(assignment).await {
                        error!(error = ?e, "Task execution failed");
                    }
                }

                AgentCommand::ProvideContext(context) => {
                    info!(task_id = ?context.task_id, "Received context");
                    if let Err(e) = self.handle_context_received(context).await {
                        error!(error = ?e, "Failed to handle context");
                    }
                }

                AgentCommand::GrantApproval => {
                    info!("Received approval");
                    if let Err(e) = self.handle_approval_granted().await {
                        error!(error = ?e, "Failed to handle approval");
                    }
                }

                AgentCommand::DenyApproval { reason } => {
                    info!(reason = %reason, "Approval denied");
                    if let Err(e) = self.handle_approval_denied(&reason).await {
                        error!(error = ?e, "Failed to handle denial");
                    }
                }
            }
        }

        info!("Agent run loop complete");
        Ok(())
    }

    /// Handle shutdown command
    async fn handle_shutdown(&mut self) -> Result<(), AgentError> {
        // If we have a current task, fail it
        if self.current_task.is_some() {
            let task_id = self.fail_task()?;
            warn!(task_id = ?task_id, "Task failed due to shutdown");

            // Notify dispatcher
            self.send_response(AgentResponse::TaskFailed {
                agent_id: self.id.clone(),
                result: TaskResult {
                    task_id,
                    status: TaskStatus::Failed,
                    output: "Agent shutdown during task execution".to_string(),
                    files_modified: vec![],
                    errors: vec!["Agent shutdown".to_string()],
                },
            })
            .await?;
        }

        // Mark as shutdown
        self.shutdown()?;

        // Send shutdown complete
        self.send_response(AgentResponse::ShutdownComplete {
            agent_id: self.id.clone(),
        })
        .await?;

        Ok(())
    }

    /// Handle a task assignment
    async fn handle_task_assignment(
        &mut self,
        assignment: TaskAssignment,
    ) -> Result<(), AgentError> {
        let task_id = assignment.task_id;

        // Transition to working state
        self.start_task(task_id)?;

        // Notify that we've started
        self.send_response(AgentResponse::TaskStarted {
            agent_id: self.id.clone(),
            task_id,
        })
        .await?;

        // Execute the task with timeout and progress updates
        match self.execute_task_with_timeout(&assignment).await {
            Ok(result) => {
                // Complete the task
                self.complete_task()?;

                self.send_response(AgentResponse::TaskCompleted {
                    agent_id: self.id.clone(),
                    result,
                })
                .await?;
            }
            Err(e) => {
                // Fail the task
                let task_id = self.fail_task()?;

                self.send_response(AgentResponse::TaskFailed {
                    agent_id: self.id.clone(),
                    result: TaskResult {
                        task_id,
                        status: TaskStatus::Failed,
                        output: String::new(),
                        files_modified: vec![],
                        errors: vec![e.to_string()],
                    },
                })
                .await?;
            }
        }

        Ok(())
    }

    /// Execute a task by calling the LLM with role-aware context
    async fn execute_task(&self, assignment: &TaskAssignment) -> Result<TaskResult, AgentError> {
        // Get role context from the assignment
        let role_context = &assignment.context.role_context;

        // Build the system prompt using role's system_prompt
        let system_prompt = self.build_role_aware_prompt(assignment, role_context);

        // Build the LLM request with role-appropriate temperature
        let temperature = self.temperature_for_style(&role_context.style);

        let request = LLMRequest {
            model: self.model_config.model_id.clone(),
            system: Some(system_prompt),
            messages: vec![Message::user(format!(
                "Please complete this task:\n\n{}\n\n{}",
                assignment.title, assignment.description
            ))],
            max_tokens: self.model_config.max_tokens,
            temperature,
            stream: false, // Will enable streaming in next slice
        };

        // Call the LLM
        let response = self
            .llm_provider()
            .send_message(request)
            .await
            .map_err(|e| AgentError::LLMError(e.to_string()))?;

        // Parse the response based on role's expected output_format
        self.parse_llm_response(assignment.task_id, response, &role_context.output_format)
    }

    /// Build system prompt with role context and required reading
    fn build_role_aware_prompt(
        &self,
        assignment: &TaskAssignment,
        role_context: &RoleContext,
    ) -> String {
        let mut prompt = role_context.system_prompt.clone();

        // Add conventions
        if !assignment.context.conventions.is_empty() {
            prompt.push_str("\n\n## Project Conventions\n");
            prompt.push_str(&assignment.context.conventions);
        }

        // Add required_reading files (loaded based on role)
        if !assignment.context.required_reading.is_empty() {
            prompt.push_str("\n\n## Required Reading\n");
            prompt.push_str("The following files are essential context for your role:\n\n");
            for file in &assignment.context.required_reading {
                prompt.push_str(&format!(
                    "### {}\n```\n{}\n```\n\n",
                    file.path, file.content
                ));
            }
        }

        // Add additional task-specific files
        if !assignment.context.files.is_empty() {
            prompt.push_str("\n\n## Task-Specific Files\n");
            for file in &assignment.context.files {
                prompt.push_str(&format!(
                    "### {}\n```\n{}\n```\n\n",
                    file.path, file.content
                ));
            }
        }

        // Add output format instructions
        prompt.push_str(&self.output_format_instructions(&role_context.output_format));

        prompt
    }

    /// Get temperature based on communication style
    fn temperature_for_style(&self, style: &CommunicationStyle) -> f32 {
        match style {
            CommunicationStyle::Technical => 0.3, // More deterministic
            CommunicationStyle::Casual => 0.7,    // More creative
            CommunicationStyle::Formal => 0.4,    // Balanced but precise
            CommunicationStyle::Friendly => 0.6,  // Warm but focused
        }
    }

    /// Generate output format instructions based on role's expected format
    fn output_format_instructions(&self, format: &OutputFormat) -> String {
        match format {
            OutputFormat::CodeAndReport => {
                "\n\n## Output Format\nProvide code with clear comments. Include only necessary explanations.".to_string()
            }
            OutputFormat::Plan => {
                "\n\n## Output Format\nProvide a structured plan with numbered steps and clear dependencies.".to_string()
            }
            OutputFormat::Summary => {
                "\n\n## Output Format\nProvide a detailed summary with sections, findings, and recommendations.".to_string()
            }
            OutputFormat::Result => {
                "\n\n## Output Format\nProvide a concise result with the key outcome clearly stated.".to_string()
            }
            OutputFormat::Custom(format) => {
                format!("\n\n## Output Format\n{}", format)
            }
        }
    }

    /// Parse LLM response based on expected output format
    fn parse_llm_response(
        &self,
        task_id: Uuid,
        response: LLMResponse,
        output_format: &OutputFormat,
    ) -> Result<TaskResult, AgentError> {
        // Parse based on expected format
        // For now, treat the entire response as output
        // Structured output parsing will be improved in M4
        let files_modified = match output_format {
            OutputFormat::CodeAndReport => self.extract_file_modifications(&response.content),
            _ => vec![],
        };

        Ok(TaskResult {
            task_id,
            status: TaskStatus::Completed,
            output: response.content,
            files_modified,
            errors: vec![],
        })
    }

    /// Extract file modifications from code output (basic implementation)
    fn extract_file_modifications(&self, content: &str) -> Vec<String> {
        // Look for file path patterns in code blocks
        // This is a basic implementation - M4 will have structured output
        let mut files = Vec::new();
        for line in content.lines() {
            if line.starts_with("// File: ") || line.starts_with("# File: ") {
                if let Some(path) = line.split(": ").nth(1) {
                    files.push(path.trim().to_string());
                }
            }
        }
        files
    }

    /// Send a progress update to the feed
    async fn emit_progress(
        &self,
        task_id: Uuid,
        message: &str,
        progress_percent: Option<u8>,
    ) -> Result<(), AgentError> {
        self.send_response(AgentResponse::ProgressUpdate {
            agent_id: self.id.clone(),
            update: ProgressUpdate {
                task_id,
                message: message.to_string(),
                progress_percent,
            },
        })
        .await
    }

    /// Execute task with progress updates
    async fn execute_task_with_progress(
        &self,
        assignment: &TaskAssignment,
    ) -> Result<TaskResult, AgentError> {
        let task_id = assignment.task_id;

        // Progress: Starting
        self.emit_progress(
            task_id,
            &format!("Starting work on: {}", assignment.title),
            Some(0),
        )
        .await?;

        // Progress: Building prompt
        self.emit_progress(task_id, "Analyzing task and building context...", Some(10))
            .await?;

        // Build prompt using role context
        let role_context = &assignment.context.role_context;
        let system_prompt = self.build_role_aware_prompt(assignment, role_context);
        let temperature = self.temperature_for_style(&role_context.style);

        // Progress: Calling LLM
        self.emit_progress(task_id, "Generating solution...", Some(30))
            .await?;

        // Build and send LLM request with role-based temperature
        let request = LLMRequest {
            model: self.model_config.model_id.clone(),
            system: Some(system_prompt),
            messages: vec![Message::user(format!(
                "Please complete this task:\n\n{}\n\n{}",
                assignment.title, assignment.description
            ))],
            max_tokens: self.model_config.max_tokens,
            temperature,
            stream: false,
        };

        let response = self
            .llm_provider()
            .send_message(request)
            .await
            .map_err(|e| AgentError::LLMError(e.to_string()))?;

        // Progress: Processing response
        self.emit_progress(task_id, "Processing results...", Some(80))
            .await?;

        // Parse response
        let result = self.parse_llm_response(task_id, response, &role_context.output_format)?;

        // Progress: Complete
        self.emit_progress(task_id, "Task complete", Some(100))
            .await?;

        Ok(result)
    }

    /// Execute task with timeout
    async fn execute_task_with_timeout(
        &self,
        assignment: &TaskAssignment,
    ) -> Result<TaskResult, AgentError> {
        let task_timeout = assignment.timeout;

        match timeout(task_timeout, self.execute_task_with_progress(assignment)).await {
            Ok(result) => result,
            Err(_) => Err(AgentError::TaskTimeout {
                task_id: assignment.task_id,
                timeout: task_timeout,
            }),
        }
    }

    /// Handle context received while waiting
    async fn handle_context_received(
        &mut self,
        context: ContextResponse,
    ) -> Result<(), AgentError> {
        // Verify we were waiting for context
        if !matches!(self.status, AgentStatus::WaitingForContext) {
            warn!("Received context but not waiting for it");
            return Ok(());
        }

        // Store the context and resume
        // In practice, this would continue the paused task
        self.resume()?;

        self.emit_progress(
            context.task_id,
            "Received additional context, resuming...",
            None,
        )
        .await?;

        // Continue task execution would happen here
        // For now, just acknowledge
        Ok(())
    }

    /// Handle approval granted
    async fn handle_approval_granted(&mut self) -> Result<(), AgentError> {
        if !matches!(self.status, AgentStatus::WaitingForApproval) {
            warn!("Received approval but not waiting for it");
            return Ok(());
        }

        self.resume()?;

        if let Some(task_id) = self.current_task {
            self.emit_progress(task_id, "Approval granted, proceeding...", None)
                .await?;
        }

        Ok(())
    }

    /// Handle approval denied
    async fn handle_approval_denied(&mut self, reason: &str) -> Result<(), AgentError> {
        if !matches!(self.status, AgentStatus::WaitingForApproval) {
            warn!("Received denial but not waiting for it");
            return Ok(());
        }

        let task_id = self.fail_task()?;

        self.send_response(AgentResponse::TaskFailed {
            agent_id: self.id.clone(),
            result: TaskResult {
                task_id,
                status: TaskStatus::Failed,
                output: String::new(),
                files_modified: vec![],
                errors: vec![format!("Approval denied: {}", reason)],
            },
        })
        .await?;

        Ok(())
    }

    /// Request additional context from dispatcher
    #[allow(dead_code)]
    async fn request_context(
        &mut self,
        task_id: Uuid,
        files_needed: Vec<String>,
        questions: Vec<String>,
    ) -> Result<(), AgentError> {
        self.wait_for_context()?;

        self.send_response(AgentResponse::ContextRequest {
            agent_id: self.id.clone(),
            request: ContextRequest {
                task_id,
                files_needed,
                questions,
            },
        })
        .await?;

        Ok(())
    }

    /// Request approval before proceeding
    #[allow(dead_code)]
    async fn request_approval(
        &mut self,
        task_id: Uuid,
        action: &str,
        details: &str,
    ) -> Result<(), AgentError> {
        self.wait_for_approval()?;

        self.send_response(AgentResponse::ApprovalRequest {
            agent_id: self.id.clone(),
            request: ApprovalRequest {
                task_id,
                action: action.to_string(),
                details: details.to_string(),
            },
        })
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::agent::Agent;
    use crate::agents::channels::*;
    use crate::agents::roles::{CommunicationStyle, OutputFormat, RoleId};
    use crate::llm::{LLMProvider, LLMRequest, LLMResponse, StopReason, StreamChunk, TokenUsage};
    use crate::types::{AgentPersona, AgentStatus, AgentTier, ModelConfig, TaskStatus};
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    struct MockLLMProvider;

    #[async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn send_message(
            &self,
            _request: LLMRequest,
        ) -> Result<LLMResponse, crate::llm::LLMError> {
            Ok(LLMResponse {
                content: "test response".to_string(),
                model: "test-model".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 20,
                },
            })
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<StreamChunk, crate::llm::LLMError>> + Send>>,
            crate::llm::LLMError,
        > {
            unimplemented!()
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    /// Mock that always errors
    struct FailingLLMProvider;

    #[async_trait]
    impl LLMProvider for FailingLLMProvider {
        async fn send_message(
            &self,
            _request: LLMRequest,
        ) -> Result<LLMResponse, crate::llm::LLMError> {
            Err(crate::llm::LLMError::ApiError {
                status: 500,
                message: "mock error".to_string(),
            })
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<StreamChunk, crate::llm::LLMError>> + Send>>,
            crate::llm::LLMError,
        > {
            unimplemented!()
        }

        fn provider_name(&self) -> &'static str {
            "mock-fail"
        }

        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    /// Slow mock for timeout testing
    struct SlowLLMProvider;

    #[async_trait]
    impl LLMProvider for SlowLLMProvider {
        async fn send_message(
            &self,
            _request: LLMRequest,
        ) -> Result<LLMResponse, crate::llm::LLMError> {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok(LLMResponse {
                content: "too slow".to_string(),
                model: "test-model".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                },
            })
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<StreamChunk, crate::llm::LLMError>> + Send>>,
            crate::llm::LLMError,
        > {
            unimplemented!()
        }

        fn provider_name(&self) -> &'static str {
            "mock-slow"
        }

        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    fn create_test_agent_with_channels(
        provider: Arc<dyn LLMProvider + Send + Sync>,
    ) -> (
        Agent,
        mpsc::Sender<AgentCommand>,
        mpsc::Receiver<AgentResponse>,
    ) {
        let (command_tx, command_rx) = mpsc::channel(32);
        let (response_tx, response_rx) = mpsc::channel(32);
        let agent = Agent::new(
            AgentTier::Worker,
            AgentPersona::default(),
            ModelConfig::default(),
            provider,
            command_rx,
            response_tx,
        );
        (agent, command_tx, response_rx)
    }

    fn create_test_agent() -> Agent {
        let (agent, _tx, _rx) = create_test_agent_with_channels(Arc::new(MockLLMProvider));
        agent
    }

    fn make_role_context() -> super::RoleContext {
        super::RoleContext {
            system_prompt: "You are a test agent.".to_string(),
            style: CommunicationStyle::Technical,
            output_format: OutputFormat::CodeAndReport,
        }
    }

    fn make_assignment() -> TaskAssignment {
        TaskAssignment {
            task_id: Uuid::new_v4(),
            title: "Test task".to_string(),
            description: "Do the thing".to_string(),
            context: TaskContext {
                required_reading: vec![],
                files: vec![],
                history: vec![],
                conventions: String::new(),
                role_context: make_role_context(),
            },
            constraints: TaskConstraints::default(),
            timeout: Duration::from_secs(30),
            role_id: RoleId::new("worker"),
        }
    }

    // === Pure function tests ===

    #[test]
    fn temperature_for_style_values() {
        let agent = create_test_agent();
        assert_eq!(agent.temperature_for_style(&CommunicationStyle::Technical), 0.3);
        assert_eq!(agent.temperature_for_style(&CommunicationStyle::Casual), 0.7);
        assert_eq!(agent.temperature_for_style(&CommunicationStyle::Formal), 0.4);
        assert_eq!(agent.temperature_for_style(&CommunicationStyle::Friendly), 0.6);
    }

    #[test]
    fn output_format_instructions_all_variants() {
        let agent = create_test_agent();

        let code = agent.output_format_instructions(&OutputFormat::CodeAndReport);
        assert!(code.contains("code"));

        let plan = agent.output_format_instructions(&OutputFormat::Plan);
        assert!(plan.contains("plan"));

        let summary = agent.output_format_instructions(&OutputFormat::Summary);
        assert!(summary.contains("summary"));

        let result = agent.output_format_instructions(&OutputFormat::Result);
        assert!(result.contains("result"));

        let custom = agent.output_format_instructions(&OutputFormat::Custom("Use YAML".to_string()));
        assert!(custom.contains("Use YAML"));
    }

    #[test]
    fn extract_file_modifications_finds_files() {
        let agent = create_test_agent();

        let content = "Some text\n// File: src/main.rs\ncode here\n# File: README.md\nmore";
        let files = agent.extract_file_modifications(content);
        assert_eq!(files, vec!["src/main.rs", "README.md"]);
    }

    #[test]
    fn extract_file_modifications_empty_when_no_markers() {
        let agent = create_test_agent();
        let files = agent.extract_file_modifications("just some plain text\nno markers here");
        assert!(files.is_empty());
    }

    #[test]
    fn parse_llm_response_code_and_report() {
        let agent = create_test_agent();
        let task_id = Uuid::new_v4();
        let response = LLMResponse {
            content: "// File: src/lib.rs\nfn main() {}".to_string(),
            model: "test".to_string(),
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage { input_tokens: 5, output_tokens: 10 },
        };

        let result = agent
            .parse_llm_response(task_id, response, &OutputFormat::CodeAndReport)
            .unwrap();
        assert_eq!(result.task_id, task_id);
        assert_eq!(result.status, TaskStatus::Completed);
        assert_eq!(result.files_modified, vec!["src/lib.rs"]);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn parse_llm_response_non_code_format_no_files() {
        let agent = create_test_agent();
        let task_id = Uuid::new_v4();
        let response = LLMResponse {
            content: "// File: src/lib.rs\nsome output".to_string(),
            model: "test".to_string(),
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage { input_tokens: 5, output_tokens: 10 },
        };

        // Plan format should not extract files
        let result = agent.parse_llm_response(task_id, response, &OutputFormat::Plan).unwrap();
        assert!(result.files_modified.is_empty());
    }

    #[test]
    fn build_role_aware_prompt_minimal() {
        let agent = create_test_agent();
        let assignment = make_assignment();
        let role_context = make_role_context();

        let prompt = agent.build_role_aware_prompt(&assignment, &role_context);
        assert!(prompt.contains("You are a test agent."));
        assert!(prompt.contains("Output Format"));
        // No conventions or files, so those sections should be absent
        assert!(!prompt.contains("## Project Conventions"));
        assert!(!prompt.contains("## Required Reading"));
        assert!(!prompt.contains("## Task-Specific Files"));
    }

    #[test]
    fn build_role_aware_prompt_with_conventions() {
        let agent = create_test_agent();
        let mut assignment = make_assignment();
        assignment.context.conventions = "Use snake_case everywhere".to_string();
        let role_context = make_role_context();

        let prompt = agent.build_role_aware_prompt(&assignment, &role_context);
        assert!(prompt.contains("## Project Conventions"));
        assert!(prompt.contains("Use snake_case everywhere"));
    }

    #[test]
    fn build_role_aware_prompt_with_required_reading() {
        let agent = create_test_agent();
        let mut assignment = make_assignment();
        assignment.context.required_reading = vec![FileContent {
            path: "CONVENTIONS.md".to_string(),
            content: "Be consistent".to_string(),
        }];
        let role_context = make_role_context();

        let prompt = agent.build_role_aware_prompt(&assignment, &role_context);
        assert!(prompt.contains("## Required Reading"));
        assert!(prompt.contains("CONVENTIONS.md"));
        assert!(prompt.contains("Be consistent"));
    }

    #[test]
    fn build_role_aware_prompt_with_task_files() {
        let agent = create_test_agent();
        let mut assignment = make_assignment();
        assignment.context.files = vec![FileContent {
            path: "src/main.rs".to_string(),
            content: "fn main() {}".to_string(),
        }];
        let role_context = make_role_context();

        let prompt = agent.build_role_aware_prompt(&assignment, &role_context);
        assert!(prompt.contains("## Task-Specific Files"));
        assert!(prompt.contains("src/main.rs"));
    }

    // === Async handler tests ===

    #[tokio::test]
    async fn handle_shutdown_idle_sends_shutdown_complete() {
        let (mut agent, _cmd_tx, mut resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        agent.handle_shutdown().await.unwrap();
        assert!(agent.is_shutdown());

        let resp = resp_rx.recv().await.unwrap();
        match resp {
            AgentResponse::ShutdownComplete { .. } => {}
            other => panic!("expected ShutdownComplete, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_shutdown_working_fails_task_then_shuts_down() {
        let (mut agent, _cmd_tx, mut resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        let task_id = Uuid::new_v4();
        agent.start_task(task_id).unwrap();

        agent.handle_shutdown().await.unwrap();
        assert!(agent.is_shutdown());

        // Should get TaskFailed then ShutdownComplete
        let resp1 = resp_rx.recv().await.unwrap();
        match resp1 {
            AgentResponse::TaskFailed { result, .. } => {
                assert_eq!(result.task_id, task_id);
                assert_eq!(result.status, TaskStatus::Failed);
            }
            other => panic!("expected TaskFailed, got {:?}", other),
        }

        let resp2 = resp_rx.recv().await.unwrap();
        match resp2 {
            AgentResponse::ShutdownComplete { .. } => {}
            other => panic!("expected ShutdownComplete, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn handle_task_assignment_success() {
        let (mut agent, _cmd_tx, mut resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        let assignment = make_assignment();
        let task_id = assignment.task_id;

        agent.handle_task_assignment(assignment).await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Idle); // completed

        // TaskStarted
        let resp1 = resp_rx.recv().await.unwrap();
        match resp1 {
            AgentResponse::TaskStarted { task_id: tid, .. } => assert_eq!(tid, task_id),
            other => panic!("expected TaskStarted, got {:?}", other),
        }

        // Progress updates (multiple), then TaskCompleted
        let mut got_completed = false;
        while let Ok(resp) = resp_rx.try_recv() {
            if let AgentResponse::TaskCompleted { result, .. } = resp {
                assert_eq!(result.task_id, task_id);
                assert_eq!(result.status, TaskStatus::Completed);
                got_completed = true;
            }
        }
        assert!(got_completed);
    }

    #[tokio::test]
    async fn handle_task_assignment_llm_failure() {
        let (mut agent, _cmd_tx, mut resp_rx) =
            create_test_agent_with_channels(Arc::new(FailingLLMProvider));

        let assignment = make_assignment();
        let task_id = assignment.task_id;

        agent.handle_task_assignment(assignment).await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Idle); // failed back to idle

        // TaskStarted
        let resp1 = resp_rx.recv().await.unwrap();
        assert!(matches!(resp1, AgentResponse::TaskStarted { .. }));

        // Drain progress, find TaskFailed
        let mut got_failed = false;
        while let Ok(resp) = resp_rx.try_recv() {
            if let AgentResponse::TaskFailed { result, .. } = resp {
                assert_eq!(result.task_id, task_id);
                assert!(!result.errors.is_empty());
                got_failed = true;
            }
        }
        assert!(got_failed);
    }

    #[tokio::test]
    async fn handle_task_assignment_timeout() {
        let (mut agent, _cmd_tx, mut resp_rx) =
            create_test_agent_with_channels(Arc::new(SlowLLMProvider));

        let mut assignment = make_assignment();
        assignment.timeout = Duration::from_millis(50); // very short timeout
        let task_id = assignment.task_id;

        agent.handle_task_assignment(assignment).await.unwrap();

        // Drain to find TaskFailed with timeout error
        let mut got_failed = false;
        while let Some(resp) = resp_rx.recv().await {
            if let AgentResponse::TaskFailed { result, .. } = resp {
                assert_eq!(result.task_id, task_id);
                assert!(result.errors.iter().any(|e| e.contains("timed out")));
                got_failed = true;
                break;
            }
        }
        assert!(got_failed);
    }

    #[tokio::test]
    async fn handle_context_received_not_waiting_is_noop() {
        let (mut agent, _cmd_tx, _resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        // Agent is Idle, not WaitingForContext
        let context = ContextResponse {
            task_id: Uuid::new_v4(),
            files: vec![],
            answers: vec![],
        };
        agent.handle_context_received(context).await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Idle);
    }

    #[tokio::test]
    async fn handle_context_received_resumes() {
        let (mut agent, _cmd_tx, mut _resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        let task_id = Uuid::new_v4();
        agent.start_task(task_id).unwrap();
        agent.wait_for_context().unwrap();
        assert_eq!(agent.status(), AgentStatus::WaitingForContext);

        let context = ContextResponse {
            task_id,
            files: vec![],
            answers: vec![],
        };
        agent.handle_context_received(context).await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Working);
    }

    #[tokio::test]
    async fn handle_approval_granted_not_waiting_is_noop() {
        let (mut agent, _cmd_tx, _resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        agent.handle_approval_granted().await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Idle);
    }

    #[tokio::test]
    async fn handle_approval_granted_resumes() {
        let (mut agent, _cmd_tx, _resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        let task_id = Uuid::new_v4();
        agent.start_task(task_id).unwrap();
        agent.wait_for_approval().unwrap();

        agent.handle_approval_granted().await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Working);
    }

    #[tokio::test]
    async fn handle_approval_denied_not_waiting_is_noop() {
        let (mut agent, _cmd_tx, _resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        agent.handle_approval_denied("nope").await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Idle);
    }

    #[tokio::test]
    async fn handle_approval_denied_fails_task() {
        let (mut agent, _cmd_tx, mut resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        let task_id = Uuid::new_v4();
        agent.start_task(task_id).unwrap();
        agent.wait_for_approval().unwrap();

        agent.handle_approval_denied("not approved").await.unwrap();
        assert_eq!(agent.status(), AgentStatus::Idle);

        let resp = resp_rx.recv().await.unwrap();
        match resp {
            AgentResponse::TaskFailed { result, .. } => {
                assert_eq!(result.task_id, task_id);
                assert!(result.errors[0].contains("not approved"));
            }
            other => panic!("expected TaskFailed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn run_loop_shutdown_command() {
        let (agent, cmd_tx, mut resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        let handle = tokio::spawn(agent.run());

        cmd_tx.send(AgentCommand::Shutdown).await.unwrap();

        handle.await.unwrap().unwrap();

        let resp = resp_rx.recv().await.unwrap();
        assert!(matches!(resp, AgentResponse::ShutdownComplete { .. }));
    }

    #[tokio::test]
    async fn run_loop_channel_closed() {
        let (agent, cmd_tx, _resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        drop(cmd_tx); // close command channel

        let result = agent.run().await;
        assert!(result.is_ok()); // graceful exit
    }

    #[tokio::test]
    async fn emit_progress_sends_update() {
        let (mut agent, _cmd_tx, mut resp_rx) =
            create_test_agent_with_channels(Arc::new(MockLLMProvider));

        let task_id = Uuid::new_v4();
        agent.emit_progress(task_id, "working...", Some(50)).await.unwrap();

        let resp = resp_rx.recv().await.unwrap();
        match resp {
            AgentResponse::ProgressUpdate { update, .. } => {
                assert_eq!(update.task_id, task_id);
                assert_eq!(update.message, "working...");
                assert_eq!(update.progress_percent, Some(50));
            }
            other => panic!("expected ProgressUpdate, got {:?}", other),
        }
    }
}
