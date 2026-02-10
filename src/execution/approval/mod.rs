//! Approval gates for dangerous operations

use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::fmt;
use std::sync::Mutex;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ApprovalError {
    #[error("approval denied by user")]
    Denied,

    #[error("approval request timed out")]
    Timeout,

    #[error("approval channel closed")]
    ChannelClosed,

    #[error("no approval handler configured")]
    NoHandler,
}

/// Categories of operations that may require approval
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DangerousOperation {
    // File operations
    DeleteFile,
    ModifyFile,
    CreateFile,

    // Git operations
    GitCommit,
    GitPush,
    GitForcePush,
    GitBranchDelete,
    GitMerge,

    // External operations
    RunCommand,
    NetworkAccess,
    InstallPackage,

    // PR/Issue operations
    CreatePullRequest,
    MergePullRequest,
    CloseIssue,
}

impl fmt::Display for DangerousOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DangerousOperation::DeleteFile => write!(f, "Delete file"),
            DangerousOperation::ModifyFile => write!(f, "Modify file"),
            DangerousOperation::CreateFile => write!(f, "Create file"),
            DangerousOperation::GitCommit => write!(f, "Create git commit"),
            DangerousOperation::GitPush => write!(f, "Push to remote"),
            DangerousOperation::GitForcePush => write!(f, "Force push to remote"),
            DangerousOperation::GitBranchDelete => write!(f, "Delete git branch"),
            DangerousOperation::GitMerge => write!(f, "Merge branch"),
            DangerousOperation::RunCommand => write!(f, "Run shell command"),
            DangerousOperation::NetworkAccess => write!(f, "Access network"),
            DangerousOperation::InstallPackage => write!(f, "Install package"),
            DangerousOperation::CreatePullRequest => write!(f, "Create pull request"),
            DangerousOperation::MergePullRequest => write!(f, "Merge pull request"),
            DangerousOperation::CloseIssue => write!(f, "Close issue"),
        }
    }
}

/// Danger level determines default behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DangerLevel {
    /// Low risk - usually auto-approved
    Low,
    /// Medium risk - approval depends on config
    Medium,
    /// High risk - usually requires approval
    High,
    /// Critical - always requires approval
    Critical,
}

impl DangerousOperation {
    /// Get the danger level for this operation
    pub fn danger_level(&self) -> DangerLevel {
        match self {
            // Low risk
            DangerousOperation::CreateFile => DangerLevel::Low,
            DangerousOperation::ModifyFile => DangerLevel::Low,

            // Medium risk
            DangerousOperation::DeleteFile => DangerLevel::Medium,
            DangerousOperation::GitCommit => DangerLevel::Medium,
            DangerousOperation::RunCommand => DangerLevel::Medium,
            DangerousOperation::CreatePullRequest => DangerLevel::Medium,

            // High risk
            DangerousOperation::GitPush => DangerLevel::High,
            DangerousOperation::GitBranchDelete => DangerLevel::High,
            DangerousOperation::GitMerge => DangerLevel::High,
            DangerousOperation::NetworkAccess => DangerLevel::High,
            DangerousOperation::InstallPackage => DangerLevel::High,
            DangerousOperation::MergePullRequest => DangerLevel::High,
            DangerousOperation::CloseIssue => DangerLevel::High,

            // Critical - always needs approval
            DangerousOperation::GitForcePush => DangerLevel::Critical,
        }
    }

    /// Human-readable description of what this operation does
    pub fn description(&self) -> &'static str {
        match self {
            DangerousOperation::DeleteFile => "Permanently remove a file from the project",
            DangerousOperation::ModifyFile => "Change the contents of an existing file",
            DangerousOperation::CreateFile => "Create a new file in the project",
            DangerousOperation::GitCommit => "Create a new commit with staged changes",
            DangerousOperation::GitPush => "Push commits to the remote repository",
            DangerousOperation::GitForcePush => {
                "Force push, potentially overwriting remote history"
            }
            DangerousOperation::GitBranchDelete => "Delete a git branch",
            DangerousOperation::GitMerge => "Merge one branch into another",
            DangerousOperation::RunCommand => "Execute a shell command",
            DangerousOperation::NetworkAccess => "Make a network request",
            DangerousOperation::InstallPackage => "Install a package or dependency",
            DangerousOperation::CreatePullRequest => "Create a new pull request on GitHub",
            DangerousOperation::MergePullRequest => "Merge a pull request into the target branch",
            DangerousOperation::CloseIssue => "Close a GitHub issue",
        }
    }
}

/// Configuration for which operations require approval
#[derive(Debug, Clone, Default)]
pub struct ApprovalGatesConfig {
    pub before_commit: bool,
    pub before_push: Option<bool>,
    pub before_pr: bool,
    pub before_merge: bool,
}

/// Autonomy level determines overall approval behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutonomyLevel {
    /// No approvals needed - full auto
    FullAuto,
    /// Only check configured gates
    #[default]
    ApprovalGates,
    /// Require approval for everything
    Supervised,
}

/// Approval gate that checks if operations need approval
pub struct ApprovalGate {
    config: ApprovalGatesConfig,
    autonomy_level: AutonomyLevel,
}

impl ApprovalGate {
    pub fn new(config: ApprovalGatesConfig, autonomy_level: AutonomyLevel) -> Self {
        Self {
            config,
            autonomy_level,
        }
    }

    /// Create a fully autonomous gate (no approvals except Critical)
    pub fn full_auto() -> Self {
        Self {
            config: ApprovalGatesConfig::default(),
            autonomy_level: AutonomyLevel::FullAuto,
        }
    }

    /// Create a supervised gate (approvals for everything)
    pub fn supervised() -> Self {
        Self {
            config: ApprovalGatesConfig::default(),
            autonomy_level: AutonomyLevel::Supervised,
        }
    }

    /// Check if an operation requires approval
    pub fn requires_approval(&self, operation: DangerousOperation) -> bool {
        // Critical operations always require approval
        if operation.danger_level() == DangerLevel::Critical {
            return true;
        }

        match self.autonomy_level {
            AutonomyLevel::FullAuto => false,
            AutonomyLevel::Supervised => true,
            AutonomyLevel::ApprovalGates => self.check_gate(operation),
        }
    }

    fn check_gate(&self, operation: DangerousOperation) -> bool {
        match operation {
            DangerousOperation::GitCommit => self.config.before_commit,
            DangerousOperation::GitPush => self.config.before_push.unwrap_or(true),
            DangerousOperation::CreatePullRequest => self.config.before_pr,
            DangerousOperation::MergePullRequest => self.config.before_merge,

            // Operations not in standard config default to danger level
            _ => operation.danger_level() >= DangerLevel::High,
        }
    }

    /// Get a list of all operations that would require approval
    pub fn list_gated_operations(&self) -> Vec<DangerousOperation> {
        use DangerousOperation::*;

        let all_ops = [
            DeleteFile,
            ModifyFile,
            CreateFile,
            GitCommit,
            GitPush,
            GitForcePush,
            GitBranchDelete,
            GitMerge,
            RunCommand,
            NetworkAccess,
            InstallPackage,
            CreatePullRequest,
            MergePullRequest,
            CloseIssue,
        ];

        all_ops
            .into_iter()
            .filter(|op| self.requires_approval(*op))
            .collect()
    }
}

/// A request for user approval
#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub operation: DangerousOperation,
    pub description: String,
    pub context: ApprovalContext,
    pub danger_level: DangerLevel,
    pub created_at: DateTime<Utc>,
}

/// Additional context for the approval request
#[derive(Debug, Clone, Default)]
pub struct ApprovalContext {
    pub task_id: Option<Uuid>,
    pub agent_id: Option<String>,
    /// Files affected by this operation
    pub affected_files: Vec<String>,
    /// Additional details (e.g., commit message, branch name)
    pub details: String,
}

impl ApprovalRequest {
    pub fn new(
        operation: DangerousOperation,
        description: impl Into<String>,
        context: ApprovalContext,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            operation,
            description: description.into(),
            danger_level: operation.danger_level(),
            context,
            created_at: Utc::now(),
        }
    }

    /// Create a simple request without much context
    pub fn simple(operation: DangerousOperation, description: impl Into<String>) -> Self {
        Self::new(operation, description, ApprovalContext::default())
    }

    /// Format for display in TUI
    pub fn format_for_display(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Operation: {}", self.operation));
        lines.push(format!("Risk Level: {:?}", self.danger_level));
        lines.push(String::new());
        lines.push(self.description.clone());

        if !self.context.affected_files.is_empty() {
            lines.push(String::new());
            lines.push("Affected files:".to_string());
            for file in &self.context.affected_files {
                lines.push(format!("  - {}", file));
            }
        }

        if !self.context.details.is_empty() {
            lines.push(String::new());
            lines.push(format!("Details: {}", self.context.details));
        }

        lines.join("\n")
    }
}

/// User's response to an approval request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalResponse {
    /// Approve this specific request
    Approve,
    /// Deny this specific request
    Deny,
    /// Approve this and all future similar requests
    AlwaysApprove,
    /// Deny this and all future similar requests
    AlwaysDeny,
}

impl ApprovalResponse {
    pub fn is_approved(&self) -> bool {
        matches!(
            self,
            ApprovalResponse::Approve | ApprovalResponse::AlwaysApprove
        )
    }
}

/// Channel types for TUI communication
pub type ApprovalRequestSender = mpsc::Sender<(ApprovalRequest, oneshot::Sender<ApprovalResponse>)>;
pub type ApprovalRequestReceiver =
    mpsc::Receiver<(ApprovalRequest, oneshot::Sender<ApprovalResponse>)>;

/// Create a channel pair for approval requests
pub fn approval_channel(buffer: usize) -> (ApprovalRequestSender, ApprovalRequestReceiver) {
    mpsc::channel(buffer)
}

/// Interactive approval gate that communicates with TUI
pub struct InteractiveApprovalGate {
    gate: ApprovalGate,
    request_tx: ApprovalRequestSender,
    /// Operations that were marked "always approve"
    always_approved: Mutex<HashSet<DangerousOperation>>,
    /// Operations that were marked "always deny"
    always_denied: Mutex<HashSet<DangerousOperation>>,
}

impl InteractiveApprovalGate {
    pub fn new(gate: ApprovalGate, request_tx: ApprovalRequestSender) -> Self {
        Self {
            gate,
            request_tx,
            always_approved: Mutex::new(HashSet::new()),
            always_denied: Mutex::new(HashSet::new()),
        }
    }

    /// Request approval for an operation
    pub async fn request_approval(&self, request: ApprovalRequest) -> Result<bool, ApprovalError> {
        // Check "always" lists first
        {
            let always_approved = self.always_approved.lock().unwrap();
            if always_approved.contains(&request.operation) {
                tracing::debug!(
                    operation = %request.operation,
                    "Auto-approved (always approve)"
                );
                return Ok(true);
            }
        }

        {
            let always_denied = self.always_denied.lock().unwrap();
            if always_denied.contains(&request.operation) {
                tracing::debug!(
                    operation = %request.operation,
                    "Auto-denied (always deny)"
                );
                return Err(ApprovalError::Denied);
            }
        }

        // Check if approval is needed
        if !self.gate.requires_approval(request.operation) {
            tracing::debug!(
                operation = %request.operation,
                "No approval required"
            );
            return Ok(true);
        }

        // Send request to TUI and wait for response
        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send((request.clone(), response_tx))
            .await
            .map_err(|_| ApprovalError::ChannelClosed)?;

        tracing::info!(
            operation = %request.operation,
            request_id = %request.id,
            "Waiting for user approval"
        );

        // Wait for response with timeout
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(crate::constants::DEFAULT_TIMEOUT_SECS),
            response_rx,
        )
        .await
        .map_err(|_| ApprovalError::Timeout)?
        .map_err(|_| ApprovalError::ChannelClosed)?;

        // Handle "always" responses
        match response {
            ApprovalResponse::AlwaysApprove => {
                let mut always_approved = self.always_approved.lock().unwrap();
                always_approved.insert(request.operation);
            }
            ApprovalResponse::AlwaysDeny => {
                let mut always_denied = self.always_denied.lock().unwrap();
                always_denied.insert(request.operation);
            }
            _ => {}
        }

        if response.is_approved() {
            tracing::info!(
                operation = %request.operation,
                request_id = %request.id,
                "Approved by user"
            );
            Ok(true)
        } else {
            tracing::info!(
                operation = %request.operation,
                request_id = %request.id,
                "Denied by user"
            );
            Err(ApprovalError::Denied)
        }
    }

    /// Check if approval would be required (without requesting it)
    pub fn would_require_approval(&self, operation: DangerousOperation) -> bool {
        // Check always lists
        {
            let always_approved = self.always_approved.lock().unwrap();
            if always_approved.contains(&operation) {
                return false;
            }
        }

        {
            let always_denied = self.always_denied.lock().unwrap();
            if always_denied.contains(&operation) {
                return true;
            }
        }

        self.gate.requires_approval(operation)
    }
}

/// Non-interactive approval gate for testing or FullAuto mode
pub struct AutoApprovalGate {
    gate: ApprovalGate,
}

impl AutoApprovalGate {
    pub fn new(gate: ApprovalGate) -> Self {
        Self { gate }
    }

    /// Always approves unless Critical and not FullAuto
    pub fn check(&self, operation: DangerousOperation) -> Result<(), ApprovalError> {
        if self.gate.requires_approval(operation) {
            // In auto mode, only Critical operations should fail
            if operation.danger_level() == DangerLevel::Critical {
                return Err(ApprovalError::Denied);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
