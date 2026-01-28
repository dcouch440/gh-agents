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
