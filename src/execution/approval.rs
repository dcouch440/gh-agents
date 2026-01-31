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
mod tests {
    use super::*;

    #[test]
    fn danger_levels_are_ordered() {
        assert!(DangerLevel::Low < DangerLevel::Medium);
        assert!(DangerLevel::Medium < DangerLevel::High);
        assert!(DangerLevel::High < DangerLevel::Critical);
    }

    #[test]
    fn force_push_is_critical() {
        assert_eq!(
            DangerousOperation::GitForcePush.danger_level(),
            DangerLevel::Critical
        );
    }

    #[test]
    fn full_auto_only_gates_critical() {
        let gate = ApprovalGate::full_auto();

        assert!(!gate.requires_approval(DangerousOperation::GitCommit));
        assert!(!gate.requires_approval(DangerousOperation::GitPush));
        assert!(gate.requires_approval(DangerousOperation::GitForcePush)); // Critical
    }

    #[test]
    fn supervised_gates_everything() {
        let gate = ApprovalGate::supervised();

        assert!(gate.requires_approval(DangerousOperation::GitCommit));
        assert!(gate.requires_approval(DangerousOperation::CreateFile));
        assert!(gate.requires_approval(DangerousOperation::GitPush));
    }

    #[test]
    fn approval_gates_respects_config() {
        let gate = ApprovalGate::new(
            ApprovalGatesConfig {
                before_commit: false,
                before_push: Some(true),
                before_pr: true,
                before_merge: true,
            },
            AutonomyLevel::ApprovalGates,
        );

        assert!(!gate.requires_approval(DangerousOperation::GitCommit));
        assert!(gate.requires_approval(DangerousOperation::GitPush));
        assert!(gate.requires_approval(DangerousOperation::CreatePullRequest));
    }

    #[test]
    fn approval_request_formats() {
        let request = ApprovalRequest::new(
            DangerousOperation::GitPush,
            "Push 3 commits to origin/main",
            ApprovalContext {
                task_id: None,
                agent_id: Some("worker-1".to_string()),
                affected_files: vec!["src/main.rs".to_string()],
                details: "Branch: feature/test".to_string(),
            },
        );

        let display = request.format_for_display();
        assert!(display.contains("Push to remote"));
        assert!(display.contains("src/main.rs"));
        assert!(display.contains("feature/test"));
    }

    #[test]
    fn approval_response_is_approved() {
        assert!(ApprovalResponse::Approve.is_approved());
        assert!(ApprovalResponse::AlwaysApprove.is_approved());
        assert!(!ApprovalResponse::Deny.is_approved());
        assert!(!ApprovalResponse::AlwaysDeny.is_approved());
    }

    #[test]
    fn auto_approval_gate_blocks_critical() {
        let gate = AutoApprovalGate::new(ApprovalGate::full_auto());

        assert!(gate.check(DangerousOperation::GitCommit).is_ok());
        assert!(gate.check(DangerousOperation::GitPush).is_ok());
        assert!(gate.check(DangerousOperation::GitForcePush).is_err());
    }

    #[test]
    fn dangerous_operation_display_all() {
        use DangerousOperation::*;
        let ops = [
            (DeleteFile, "Delete file"),
            (ModifyFile, "Modify file"),
            (CreateFile, "Create file"),
            (GitCommit, "Create git commit"),
            (GitPush, "Push to remote"),
            (GitForcePush, "Force push to remote"),
            (GitBranchDelete, "Delete git branch"),
            (GitMerge, "Merge branch"),
            (RunCommand, "Run shell command"),
            (NetworkAccess, "Access network"),
            (InstallPackage, "Install package"),
            (CreatePullRequest, "Create pull request"),
            (MergePullRequest, "Merge pull request"),
            (CloseIssue, "Close issue"),
        ];
        for (op, expected) in &ops {
            assert_eq!(op.to_string(), *expected);
        }
    }

    #[test]
    fn approval_error_display() {
        assert_eq!(ApprovalError::Denied.to_string(), "approval denied by user");
        assert_eq!(
            ApprovalError::Timeout.to_string(),
            "approval request timed out"
        );
        assert_eq!(
            ApprovalError::ChannelClosed.to_string(),
            "approval channel closed"
        );
        assert_eq!(
            ApprovalError::NoHandler.to_string(),
            "no approval handler configured"
        );
    }

    #[test]
    fn danger_level_classification() {
        use DangerousOperation::*;
        assert_eq!(CreateFile.danger_level(), DangerLevel::Low);
        assert_eq!(ModifyFile.danger_level(), DangerLevel::Low);
        assert_eq!(DeleteFile.danger_level(), DangerLevel::Medium);
        assert_eq!(GitCommit.danger_level(), DangerLevel::Medium);
        assert_eq!(RunCommand.danger_level(), DangerLevel::Medium);
        assert_eq!(CreatePullRequest.danger_level(), DangerLevel::Medium);
        assert_eq!(GitPush.danger_level(), DangerLevel::High);
        assert_eq!(GitBranchDelete.danger_level(), DangerLevel::High);
        assert_eq!(GitMerge.danger_level(), DangerLevel::High);
        assert_eq!(NetworkAccess.danger_level(), DangerLevel::High);
        assert_eq!(InstallPackage.danger_level(), DangerLevel::High);
        assert_eq!(MergePullRequest.danger_level(), DangerLevel::High);
        assert_eq!(CloseIssue.danger_level(), DangerLevel::High);
        assert_eq!(GitForcePush.danger_level(), DangerLevel::Critical);
    }

    #[test]
    fn approval_request_simple() {
        let req = ApprovalRequest::simple(DangerousOperation::GitPush, "Push changes");
        assert_eq!(req.operation, DangerousOperation::GitPush);
        assert_eq!(req.description, "Push changes");
        assert_eq!(req.danger_level, DangerLevel::High);
        assert!(req.context.affected_files.is_empty());
    }

    #[test]
    fn approval_request_format_no_files_no_details() {
        let req = ApprovalRequest::simple(DangerousOperation::GitCommit, "Commit");
        let display = req.format_for_display();
        assert!(display.contains("Create git commit"));
        assert!(display.contains("Commit"));
        assert!(!display.contains("Affected files"));
        assert!(!display.contains("Details"));
    }

    #[test]
    fn auto_approval_gate_with_supervised_rejects_non_critical() {
        let gate = AutoApprovalGate::new(ApprovalGate::supervised());
        // Supervised requires approval for everything, but AutoApprovalGate
        // only blocks Critical operations
        assert!(gate.check(DangerousOperation::GitCommit).is_ok());
        assert!(gate.check(DangerousOperation::GitForcePush).is_err());
    }

    #[test]
    fn dangerous_operation_description_all_variants() {
        use DangerousOperation::*;
        let ops = [
            (DeleteFile, "Permanently remove a file from the project"),
            (ModifyFile, "Change the contents of an existing file"),
            (CreateFile, "Create a new file in the project"),
            (GitCommit, "Create a new commit with staged changes"),
            (GitPush, "Push commits to the remote repository"),
            (
                GitForcePush,
                "Force push, potentially overwriting remote history",
            ),
            (GitBranchDelete, "Delete a git branch"),
            (GitMerge, "Merge one branch into another"),
            (RunCommand, "Execute a shell command"),
            (NetworkAccess, "Make a network request"),
            (InstallPackage, "Install a package or dependency"),
            (CreatePullRequest, "Create a new pull request on GitHub"),
            (
                MergePullRequest,
                "Merge a pull request into the target branch",
            ),
            (CloseIssue, "Close a GitHub issue"),
        ];
        for (op, expected) in &ops {
            assert_eq!(op.description(), *expected);
        }
    }

    #[test]
    fn list_gated_operations_full_auto() {
        let gate = ApprovalGate::full_auto();
        let gated = gate.list_gated_operations();
        // FullAuto only gates Critical ops
        assert_eq!(gated, vec![DangerousOperation::GitForcePush]);
    }

    #[test]
    fn list_gated_operations_supervised() {
        let gate = ApprovalGate::supervised();
        let gated = gate.list_gated_operations();
        // Supervised gates everything (14 operations)
        assert_eq!(gated.len(), 14);
    }

    #[test]
    fn list_gated_operations_with_config() {
        let gate = ApprovalGate::new(
            ApprovalGatesConfig {
                before_commit: true,
                before_push: Some(false),
                before_pr: false,
                before_merge: false,
            },
            AutonomyLevel::ApprovalGates,
        );
        let gated = gate.list_gated_operations();
        // Should include: GitCommit (config), all High-level defaults, GitForcePush (Critical)
        assert!(gated.contains(&DangerousOperation::GitCommit));
        assert!(!gated.contains(&DangerousOperation::GitPush)); // explicitly false
        assert!(!gated.contains(&DangerousOperation::CreatePullRequest)); // before_pr=false
        assert!(gated.contains(&DangerousOperation::GitForcePush)); // Critical
        assert!(gated.contains(&DangerousOperation::GitMerge)); // High default
    }

    #[test]
    fn check_gate_before_push_defaults_to_true() {
        let gate = ApprovalGate::new(
            ApprovalGatesConfig {
                before_commit: false,
                before_push: None, // defaults to true
                before_pr: false,
                before_merge: false,
            },
            AutonomyLevel::ApprovalGates,
        );
        assert!(gate.requires_approval(DangerousOperation::GitPush));
    }

    #[test]
    fn check_gate_non_config_ops_default_by_danger_level() {
        let gate = ApprovalGate::new(ApprovalGatesConfig::default(), AutonomyLevel::ApprovalGates);
        // Low risk ops: no approval
        assert!(!gate.requires_approval(DangerousOperation::CreateFile));
        assert!(!gate.requires_approval(DangerousOperation::ModifyFile));
        // Medium risk ops (not in config): no approval (< High)
        assert!(!gate.requires_approval(DangerousOperation::DeleteFile));
        assert!(!gate.requires_approval(DangerousOperation::RunCommand));
        // High risk ops (not in config): approval required
        assert!(gate.requires_approval(DangerousOperation::GitBranchDelete));
        assert!(gate.requires_approval(DangerousOperation::NetworkAccess));
        assert!(gate.requires_approval(DangerousOperation::InstallPackage));
        assert!(gate.requires_approval(DangerousOperation::CloseIssue));
    }

    #[test]
    fn autonomy_level_default_is_approval_gates() {
        assert_eq!(AutonomyLevel::default(), AutonomyLevel::ApprovalGates);
    }

    #[test]
    fn approval_gates_config_default() {
        let config = ApprovalGatesConfig::default();
        assert!(!config.before_commit);
        assert!(config.before_push.is_none());
        assert!(!config.before_pr);
        assert!(!config.before_merge);
    }

    #[test]
    fn approval_channel_creates_pair() {
        let (_tx, _rx) = approval_channel(10);
        // Just verify it compiles and creates without panic
    }

    #[tokio::test]
    async fn interactive_gate_auto_approves_when_not_required() {
        let (tx, _rx) = approval_channel(10);
        let gate = InteractiveApprovalGate::new(ApprovalGate::full_auto(), tx);

        let req = ApprovalRequest::simple(DangerousOperation::GitCommit, "commit");
        let result = gate.request_approval(req).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn interactive_gate_always_approve_remembered() {
        let (tx, mut rx) = approval_channel(10);
        let gate = InteractiveApprovalGate::new(ApprovalGate::supervised(), tx);

        // Spawn responder that sends AlwaysApprove
        tokio::spawn(async move {
            if let Some((_, response_tx)) = rx.recv().await {
                response_tx.send(ApprovalResponse::AlwaysApprove).unwrap();
            }
        });

        let req = ApprovalRequest::simple(DangerousOperation::GitCommit, "commit");
        let result = gate.request_approval(req).await;
        assert!(result.is_ok());

        // Second request should be auto-approved without going through channel
        let req2 = ApprovalRequest::simple(DangerousOperation::GitCommit, "commit 2");
        let result2 = gate.request_approval(req2).await;
        assert!(result2.is_ok());
        assert!(result2.unwrap());
    }

    #[tokio::test]
    async fn interactive_gate_always_deny_remembered() {
        let (tx, mut rx) = approval_channel(10);
        let gate = InteractiveApprovalGate::new(ApprovalGate::supervised(), tx);

        tokio::spawn(async move {
            if let Some((_, response_tx)) = rx.recv().await {
                response_tx.send(ApprovalResponse::AlwaysDeny).unwrap();
            }
        });

        let req = ApprovalRequest::simple(DangerousOperation::GitPush, "push");
        let result = gate.request_approval(req).await;
        assert!(result.is_err());

        // Second request should be auto-denied
        let req2 = ApprovalRequest::simple(DangerousOperation::GitPush, "push 2");
        let result2 = gate.request_approval(req2).await;
        assert!(matches!(result2, Err(ApprovalError::Denied)));
    }

    #[tokio::test]
    async fn interactive_gate_deny_returns_error() {
        let (tx, mut rx) = approval_channel(10);
        let gate = InteractiveApprovalGate::new(ApprovalGate::supervised(), tx);

        tokio::spawn(async move {
            if let Some((_, response_tx)) = rx.recv().await {
                response_tx.send(ApprovalResponse::Deny).unwrap();
            }
        });

        let req = ApprovalRequest::simple(DangerousOperation::GitCommit, "commit");
        let result = gate.request_approval(req).await;
        assert!(matches!(result, Err(ApprovalError::Denied)));
    }

    #[tokio::test]
    async fn interactive_gate_channel_closed_error() {
        let (tx, rx) = approval_channel(10);
        let gate = InteractiveApprovalGate::new(ApprovalGate::supervised(), tx);

        // Drop receiver so channel is closed
        drop(rx);

        let req = ApprovalRequest::simple(DangerousOperation::GitCommit, "commit");
        let result = gate.request_approval(req).await;
        assert!(matches!(result, Err(ApprovalError::ChannelClosed)));
    }

    #[test]
    fn interactive_gate_would_require_approval_with_always_lists() {
        let (tx, _rx) = approval_channel(10);
        let gate = InteractiveApprovalGate::new(ApprovalGate::supervised(), tx);

        // Before adding to always lists, should require approval
        assert!(gate.would_require_approval(DangerousOperation::GitCommit));

        // Add to always_approved
        {
            let mut approved = gate.always_approved.lock().unwrap();
            approved.insert(DangerousOperation::GitCommit);
        }
        assert!(!gate.would_require_approval(DangerousOperation::GitCommit));

        // Add to always_denied
        {
            let mut denied = gate.always_denied.lock().unwrap();
            denied.insert(DangerousOperation::GitPush);
        }
        assert!(gate.would_require_approval(DangerousOperation::GitPush));
    }

    #[tokio::test]
    async fn interactive_gate_approve_single_not_remembered() {
        let (tx, mut rx) = approval_channel(10);
        let gate = InteractiveApprovalGate::new(ApprovalGate::supervised(), tx);

        // Respond with single Approve twice
        tokio::spawn(async move {
            for _ in 0..2 {
                if let Some((_, response_tx)) = rx.recv().await {
                    response_tx.send(ApprovalResponse::Approve).unwrap();
                }
            }
        });

        let req = ApprovalRequest::simple(DangerousOperation::GitCommit, "commit");
        assert!(gate.request_approval(req).await.unwrap());

        // Second request should still go through channel (not auto-approved)
        let req2 = ApprovalRequest::simple(DangerousOperation::GitCommit, "commit 2");
        assert!(gate.request_approval(req2).await.unwrap());
    }

    #[tokio::test]
    async fn interactive_gate_response_rx_dropped_returns_channel_closed() {
        let (tx, mut rx) = approval_channel(10);
        let gate = InteractiveApprovalGate::new(ApprovalGate::supervised(), tx);

        // Receive the request but drop the response sender
        tokio::spawn(async move {
            if let Some((_req, response_tx)) = rx.recv().await {
                drop(response_tx);
            }
        });

        let req = ApprovalRequest::simple(DangerousOperation::GitCommit, "commit");
        let result = gate.request_approval(req).await;
        assert!(matches!(result, Err(ApprovalError::ChannelClosed)));
    }

    #[test]
    fn approval_request_format_with_details_no_files() {
        let req = ApprovalRequest::new(
            DangerousOperation::RunCommand,
            "Run tests",
            ApprovalContext {
                task_id: Some(Uuid::new_v4()),
                agent_id: None,
                affected_files: vec![],
                details: "cargo test".to_string(),
            },
        );
        let display = req.format_for_display();
        assert!(display.contains("Run shell command"));
        assert!(display.contains("Run tests"));
        assert!(display.contains("Details: cargo test"));
        assert!(!display.contains("Affected files"));
    }

    #[test]
    fn auto_approval_gate_approval_gates_mode_non_critical_passes() {
        // ApprovalGates mode with config requiring approval for GitPush (High, not Critical)
        let gate = AutoApprovalGate::new(ApprovalGate::new(
            ApprovalGatesConfig {
                before_commit: false,
                before_push: Some(true),
                before_pr: false,
                before_merge: false,
            },
            AutonomyLevel::ApprovalGates,
        ));
        // GitPush requires approval but is High (not Critical), so auto-gate allows it
        assert!(gate.check(DangerousOperation::GitPush).is_ok());
    }

    #[test]
    fn check_gate_before_merge_false() {
        let gate = ApprovalGate::new(
            ApprovalGatesConfig {
                before_commit: false,
                before_push: Some(false),
                before_pr: false,
                before_merge: false,
            },
            AutonomyLevel::ApprovalGates,
        );
        assert!(!gate.requires_approval(DangerousOperation::MergePullRequest));
    }

    #[test]
    fn approval_request_danger_level_matches_operation() {
        let req = ApprovalRequest::simple(DangerousOperation::GitForcePush, "force push");
        assert_eq!(req.danger_level, DangerLevel::Critical);

        let req2 = ApprovalRequest::simple(DangerousOperation::CreateFile, "create");
        assert_eq!(req2.danger_level, DangerLevel::Low);
    }
}
