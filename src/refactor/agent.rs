//! Refactor agent for handling mid-stream plan modifications.
//!
//! The refactor agent is a specialized persona that:
//! - Converses with users to understand desired changes
//! - Detects intent from user messages
//! - Proposes changes to planning files
//! - Coordinates with the scheduler to halt/resume production

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::db::{
    get_active_refactor_session, insert_refactor_change, insert_refactor_session,
    update_change_status, update_refactor_session,
};
use crate::orchestration::Scheduler;
use crate::prompts::templates::RefactorPrompts;
use crate::types::{
    ChangeStatus, ChangeType, RefactorChange, RefactorContext, RefactorIntent, RefactorSession,
};

/// Response from intent analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnalysis {
    /// Detected intent
    pub intent: RefactorIntent,
    /// Confidence level
    pub confidence: Confidence,
    /// Reasoning for the classification
    pub reasoning: String,
    /// Whether production should be halted
    pub should_halt_production: bool,
    /// Reason for halting (if applicable)
    pub halt_reason: Option<String>,
    /// Files that would be affected by changes
    pub affected_files: Vec<String>,
    /// Question to ask if intent is unclear
    pub clarifying_question: Option<String>,
    /// Conversational response to the user
    pub response: String,
}

/// Confidence level for intent detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// Proposed change from the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedChange {
    /// Path to the file
    pub file_path: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Reason for the change
    pub reason: String,
    /// Summary of current content
    pub before_summary: Option<String>,
    /// Summary of new content
    pub after_summary: String,
    /// Complete new file content
    pub full_content: String,
}

/// Response from change proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeProposal {
    /// Brief summary of the proposed changes
    pub summary: String,
    /// List of proposed changes
    pub changes: Vec<ProposedChange>,
    /// Whether changes affect in-progress work
    pub impacts_in_progress: bool,
    /// Details of the impact
    pub impact_details: Option<String>,
    /// Whether production should be halted
    pub requires_halt: bool,
    /// Next steps after applying changes
    pub next_steps: Vec<String>,
}

/// Impact analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    /// Whether in-progress work is affected
    pub has_impact: bool,
    /// List of affected task identifiers
    pub affected_tasks: Vec<String>,
    /// Severity of the impact
    pub impact_severity: ImpactSeverity,
    /// Explanation of why tasks are affected
    pub explanation: String,
    /// Recommended action
    pub recommendation: ImpactRecommendation,
    /// Reasoning for the recommendation
    pub reasoning: String,
}

/// Impact severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImpactSeverity {
    None,
    Low,
    Medium,
    High,
}

/// Recommended action based on impact analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactRecommendation {
    Continue,
    WaitForCheckpoint,
    HaltImmediately,
}

/// The refactor agent handles mid-stream plan modifications
pub struct RefactorAgent {
    scheduler: Arc<RwLock<Scheduler>>,
    pool: sqlx::PgPool,
    session: Option<RefactorSession>,
}

impl RefactorAgent {
    /// Create a new refactor agent
    pub fn new(scheduler: Arc<RwLock<Scheduler>>, pool: sqlx::PgPool) -> Self {
        Self {
            scheduler,
            pool,
            session: None,
        }
    }

    /// Start a new refactor session
    pub async fn start_session(&mut self) -> Result<&RefactorSession> {
        // Check for existing active session
        if let Some(existing) = get_active_refactor_session(&self.pool).await? {
            tracing::info!(session_id = %existing.id, "Resuming existing refactor session");
            self.session = Some(existing);
        } else {
            // Create new session
            let session = RefactorSession::new();
            insert_refactor_session(&self.pool, &session).await?;
            tracing::info!(session_id = %session.id, "Started new refactor session");
            self.session = Some(session);

            // Enter refactor mode in scheduler
            self.scheduler.read().await.enter_refactor_mode().await?;
        }

        Ok(self.session.as_ref().unwrap())
    }

    /// Get the current session
    pub fn session(&self) -> Option<&RefactorSession> {
        self.session.as_ref()
    }

    /// Get a mutable reference to the current session
    pub fn session_mut(&mut self) -> Option<&mut RefactorSession> {
        self.session.as_mut()
    }

    /// Analyze user message for intent (without LLM - rule-based fallback)
    ///
    /// This is a simple rule-based implementation. In production, this would
    /// call the LLM with RefactorPrompts::conversation().
    pub fn analyze_intent_simple(&self, message: &str) -> IntentAnalysis {
        let message_lower = message.to_lowercase().trim().to_string();

        // Check for HALT_NOW patterns
        let halt_patterns = ["stop", "halt", "pause everything", "stop all work"];
        if halt_patterns.iter().any(|p| message_lower.contains(p)) {
            return IntentAnalysis {
                intent: RefactorIntent::HaltNow,
                confidence: Confidence::High,
                reasoning: "User explicitly requested to stop production".to_string(),
                should_halt_production: true,
                halt_reason: Some("User requested halt".to_string()),
                affected_files: vec![],
                clarifying_question: None,
                response: "I'll halt production now. What changes do you want to make?".to_string(),
            };
        }

        // Check for EXIT_REFACTOR patterns
        let exit_patterns = ["done", "exit", "continue", "resume", "let's continue"];
        if exit_patterns.iter().any(|p| message_lower.contains(p)) {
            return IntentAnalysis {
                intent: RefactorIntent::ExitRefactor,
                confidence: Confidence::High,
                reasoning: "User wants to exit refactor mode".to_string(),
                should_halt_production: false,
                halt_reason: None,
                affected_files: vec![],
                clarifying_question: None,
                response: "Exiting refactor mode. Production will resume.".to_string(),
            };
        }

        // Check for REFACTOR_NEEDED patterns
        let refactor_patterns = [
            "change",
            "modify",
            "update",
            "restructure",
            "split",
            "merge",
            "delete",
            "remove",
            "add",
            "new ticket",
            "isn't working",
            "not working",
        ];
        if refactor_patterns.iter().any(|p| message_lower.contains(p)) {
            return IntentAnalysis {
                intent: RefactorIntent::RefactorNeeded,
                confidence: Confidence::Medium,
                reasoning: "User described changes to the plan".to_string(),
                should_halt_production: false, // Depends on impact analysis
                halt_reason: None,
                affected_files: vec![],
                clarifying_question: Some(
                    "What specific files or tickets does this affect?".to_string(),
                ),
                response: "I understand you want to make changes. Let me help you figure out what needs to be modified.".to_string(),
            };
        }

        // Check for CLARIFYING patterns
        let clarifying_patterns = ["what if", "could we", "maybe", "thinking about", "consider"];
        if clarifying_patterns
            .iter()
            .any(|p| message_lower.contains(p))
        {
            return IntentAnalysis {
                intent: RefactorIntent::Clarifying,
                confidence: Confidence::Medium,
                reasoning: "User is exploring options".to_string(),
                should_halt_production: false,
                halt_reason: None,
                affected_files: vec![],
                clarifying_question: None,
                response: "That's an interesting idea. Tell me more about what you're thinking."
                    .to_string(),
            };
        }

        // Default to JUST_CHATTING
        IntentAnalysis {
            intent: RefactorIntent::JustChatting,
            confidence: Confidence::Low,
            reasoning: "No clear refactor intent detected".to_string(),
            should_halt_production: false,
            halt_reason: None,
            affected_files: vec![],
            clarifying_question: None,
            response: "I'm in refactor mode, ready to help you modify the project plan. What would you like to change?".to_string(),
        }
    }

    /// Build the prompt for LLM-based intent analysis
    pub fn build_intent_prompt(
        &self,
        message: &str,
        conversation_history: &[(&str, &str)],
        context: &RefactorContext,
    ) -> String {
        let work_status = if context.in_progress_work.is_empty() {
            None
        } else {
            Some(context.in_progress_work.join("\n"))
        };

        let builder = RefactorPrompts::conversation(
            message,
            conversation_history,
            work_status.as_deref(),
            None, // Plan context would be loaded separately
        );

        builder.build().text
    }

    /// Halt production (pause the scheduler)
    pub async fn halt_production(&mut self) -> Result<()> {
        self.scheduler.read().await.pause_for_refactor().await?;

        if let Some(session) = &mut self.session {
            session.halt_production();
            update_refactor_session(&self.pool, session).await?;
        }

        tracing::info!("Production halted for refactor");
        Ok(())
    }

    /// Add a proposed change to the session
    pub async fn add_proposed_change(&mut self, change: RefactorChange) -> Result<()> {
        insert_refactor_change(&self.pool, &change).await?;

        if let Some(session) = &mut self.session {
            session.add_change(change);
        }

        Ok(())
    }

    /// Approve a change
    pub async fn approve_change(&mut self, change_id: &crate::types::ChangeId) -> Result<()> {
        update_change_status(&self.pool, change_id, ChangeStatus::Approved).await?;

        if let Some(session) = &mut self.session {
            for change in &mut session.proposed_changes {
                if change.id == *change_id {
                    change.status = ChangeStatus::Approved;
                    break;
                }
            }
        }

        Ok(())
    }

    /// Apply approved changes to the filesystem
    pub async fn apply_changes(&mut self, base_path: &Path) -> Result<Vec<String>> {
        let session = self
            .session
            .as_mut()
            .context("No active refactor session")?;

        let mut applied_files = Vec::new();

        for change in session.approved_changes() {
            let file_path = base_path.join(&change.file_path);

            match change.change_type {
                ChangeType::Create | ChangeType::Modify => {
                    if let Some(content) = &change.after_content {
                        // Ensure parent directory exists
                        if let Some(parent) = file_path.parent() {
                            tokio::fs::create_dir_all(parent).await?;
                        }
                        tokio::fs::write(&file_path, content).await?;
                        tracing::info!(path = %file_path.display(), "Applied change");
                    }
                }
                ChangeType::Delete => {
                    if file_path.exists() {
                        tokio::fs::remove_file(&file_path).await?;
                        tracing::info!(path = %file_path.display(), "Deleted file");
                    }
                }
                ChangeType::Rename => {
                    // For rename, after_content contains the new path
                    if let Some(new_path_str) = &change.after_content {
                        let new_path = base_path.join(new_path_str);
                        if let Some(parent) = new_path.parent() {
                            tokio::fs::create_dir_all(parent).await?;
                        }
                        tokio::fs::rename(&file_path, &new_path).await?;
                        tracing::info!(
                            from = %file_path.display(),
                            to = %new_path.display(),
                            "Renamed file"
                        );
                    }
                }
            }

            applied_files.push(change.file_path.clone());

            // Mark as applied in the database
            update_change_status(&self.pool, &change.id, ChangeStatus::Applied).await?;
        }

        session.mark_changes_applied();
        update_refactor_session(&self.pool, session).await?;

        Ok(applied_files)
    }

    /// End the refactor session and resume production
    pub async fn end_session(&mut self) -> Result<()> {
        if let Some(session) = &mut self.session {
            session.end();
            update_refactor_session(&self.pool, session).await?;
            tracing::info!(session_id = %session.id, "Ended refactor session");
        }

        // Resume production
        let scheduler = self.scheduler.read().await;
        scheduler.begin_resume().await?;
        scheduler.resume().await?;

        self.session = None;
        Ok(())
    }

    /// Get the current refactor context
    pub async fn get_context(&self) -> Result<RefactorContext> {
        let mode = self.scheduler.read().await.get_production_mode().await;
        let mut ctx = RefactorContext::new(mode);

        if let Some(session) = &self.session {
            ctx = ctx.with_session(session.clone());
        }

        // In a full implementation, we'd load in-progress work from the database
        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::TestDb;
    use crate::types::ProductionMode;
    use tempfile::TempDir;

    async fn setup_agent() -> (RefactorAgent, TestDb) {
        let db = TestDb::new().await;
        let scheduler = Scheduler::new(db.pool.clone()).await.unwrap();
        let agent = RefactorAgent::new(Arc::new(RwLock::new(scheduler)), db.pool.clone());
        (agent, db)
    }

    #[tokio::test]
    async fn agent_starts_with_no_session() {
        let (agent, db) = setup_agent().await;
        assert!(agent.session().is_none());
        db.cleanup().await;
    }

    #[tokio::test]
    async fn agent_can_start_session() {
        let (mut agent, db) = setup_agent().await;

        let session = agent.start_session().await.unwrap();
        assert!(session.is_active());
        assert!(!session.production_halted);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn analyze_intent_detects_halt() {
        let (agent, db) = setup_agent().await;

        let analysis = agent.analyze_intent_simple("STOP all work right now");
        assert_eq!(analysis.intent, RefactorIntent::HaltNow);
        assert!(analysis.should_halt_production);
        assert_eq!(analysis.confidence, Confidence::High);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn analyze_intent_detects_exit() {
        let (agent, db) = setup_agent().await;

        let analysis = agent.analyze_intent_simple("done, let's continue");
        assert_eq!(analysis.intent, RefactorIntent::ExitRefactor);
        assert!(!analysis.should_halt_production);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn analyze_intent_detects_refactor() {
        let (agent, db) = setup_agent().await;

        let analysis = agent.analyze_intent_simple("I want to change how ticket 2.3 works");
        assert_eq!(analysis.intent, RefactorIntent::RefactorNeeded);
        assert!(analysis.clarifying_question.is_some());
        db.cleanup().await;
    }

    #[tokio::test]
    async fn analyze_intent_detects_clarifying() {
        let (agent, db) = setup_agent().await;

        let analysis = agent.analyze_intent_simple("What if we approached it differently?");
        assert_eq!(analysis.intent, RefactorIntent::Clarifying);
        assert!(!analysis.should_halt_production);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn analyze_intent_defaults_to_chatting() {
        let (agent, db) = setup_agent().await;

        let analysis = agent.analyze_intent_simple("Hey, how's it going?");
        assert_eq!(analysis.intent, RefactorIntent::JustChatting);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn agent_can_halt_production() {
        let (mut agent, db) = setup_agent().await;

        agent.start_session().await.unwrap();
        agent.halt_production().await.unwrap();

        assert!(agent.session().unwrap().production_halted);

        let mode = agent.scheduler.read().await.get_production_mode().await;
        assert_eq!(mode, ProductionMode::Paused);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn agent_can_add_and_apply_changes() {
        let (mut agent, db) = setup_agent().await;
        let temp_dir = TempDir::new().unwrap();

        agent.start_session().await.unwrap();

        // Add a proposed change
        let change = RefactorChange::create(
            agent.session().unwrap().id.clone(),
            "test_file.md".to_string(),
            "# Test Content\n\nThis is a test.".to_string(),
            "Creating test file".to_string(),
        );

        let change_id = change.id.clone();
        agent.add_proposed_change(change).await.unwrap();

        // Approve the change
        agent.approve_change(&change_id).await.unwrap();

        // Apply changes
        let applied = agent.apply_changes(temp_dir.path()).await.unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], "test_file.md");

        // Verify file was created
        let content = tokio::fs::read_to_string(temp_dir.path().join("test_file.md"))
            .await
            .unwrap();
        assert!(content.contains("Test Content"));
        db.cleanup().await;
    }

    #[tokio::test]
    async fn agent_can_end_session() {
        let (mut agent, db) = setup_agent().await;

        agent.start_session().await.unwrap();
        agent.end_session().await.unwrap();

        assert!(agent.session().is_none());

        let mode = agent.scheduler.read().await.get_production_mode().await;
        assert_eq!(mode, ProductionMode::Running);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn agent_resumes_existing_session() {
        let (mut agent, db) = setup_agent().await;

        // Start a session
        let session1 = agent.start_session().await.unwrap();
        let session1_id = session1.id.clone();

        // Drop and recreate agent (simulating restart)
        let pool = agent.pool.clone();
        let scheduler = agent.scheduler.clone();
        drop(agent);

        let mut agent2 = RefactorAgent::new(scheduler, pool);
        let session2 = agent2.start_session().await.unwrap();

        // Should resume the same session
        assert_eq!(session2.id, session1_id);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn get_context_includes_session() {
        let (mut agent, db) = setup_agent().await;

        agent.start_session().await.unwrap();
        let ctx = agent.get_context().await.unwrap();

        assert!(ctx.session.is_some());
        assert!(ctx.production_mode.is_refactoring());
        db.cleanup().await;
    }

    #[tokio::test]
    async fn get_context_without_session() {
        let (agent, db) = setup_agent().await;
        let ctx = agent.get_context().await.unwrap();
        assert!(ctx.session.is_none());
        db.cleanup().await;
    }

    #[tokio::test]
    async fn session_mut_returns_mutable_ref() {
        let (mut agent, db) = setup_agent().await;
        assert!(agent.session_mut().is_none());

        agent.start_session().await.unwrap();
        let session = agent.session_mut().unwrap();
        assert!(session.is_active());
        db.cleanup().await;
    }

    #[tokio::test]
    async fn analyze_intent_halt_pattern() {
        let (agent, db) = setup_agent().await;

        let analysis = agent.analyze_intent_simple("halt everything");
        assert_eq!(analysis.intent, RefactorIntent::HaltNow);
        assert!(analysis.halt_reason.is_some());
        db.cleanup().await;
    }

    #[tokio::test]
    async fn analyze_intent_pause_everything() {
        let (agent, db) = setup_agent().await;

        let analysis = agent.analyze_intent_simple("pause everything now");
        assert_eq!(analysis.intent, RefactorIntent::HaltNow);
        assert_eq!(analysis.confidence, Confidence::High);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn analyze_intent_exit_with_resume() {
        let (agent, db) = setup_agent().await;

        let analysis = agent.analyze_intent_simple("resume work please");
        assert_eq!(analysis.intent, RefactorIntent::ExitRefactor);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn analyze_intent_refactor_various_patterns() {
        let (agent, db) = setup_agent().await;

        for msg in &[
            "delete that file",
            "add a new ticket",
            "split this up",
            "not working right",
        ] {
            let analysis = agent.analyze_intent_simple(msg);
            assert_eq!(
                analysis.intent,
                RefactorIntent::RefactorNeeded,
                "Failed for: {}",
                msg
            );
        }
        db.cleanup().await;
    }

    #[tokio::test]
    async fn analyze_intent_clarifying_patterns() {
        let (agent, db) = setup_agent().await;

        for msg in &[
            "could we try something else",
            "maybe a different approach",
            "consider this",
        ] {
            let analysis = agent.analyze_intent_simple(msg);
            assert_eq!(
                analysis.intent,
                RefactorIntent::Clarifying,
                "Failed for: {}",
                msg
            );
        }
        db.cleanup().await;
    }

    #[tokio::test]
    async fn apply_changes_no_session_errors() {
        let (mut agent, db) = setup_agent().await;
        let temp_dir = TempDir::new().unwrap();
        let result = agent.apply_changes(temp_dir.path()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No active refactor session"));
        db.cleanup().await;
    }

    #[tokio::test]
    async fn apply_modify_change() {
        let (mut agent, db) = setup_agent().await;
        let temp_dir = TempDir::new().unwrap();
        agent.start_session().await.unwrap();

        // Create an existing file to modify
        let file_path = temp_dir.path().join("existing.txt");
        tokio::fs::write(&file_path, "old content").await.unwrap();

        let change = RefactorChange::modify(
            agent.session().unwrap().id.clone(),
            "existing.txt".to_string(),
            "old content".to_string(),
            "new content".to_string(),
            "Updating file".to_string(),
        );
        let change_id = change.id.clone();
        agent.add_proposed_change(change).await.unwrap();
        agent.approve_change(&change_id).await.unwrap();

        let applied = agent.apply_changes(temp_dir.path()).await.unwrap();
        assert_eq!(applied, vec!["existing.txt"]);

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "new content");
        db.cleanup().await;
    }

    #[tokio::test]
    async fn apply_delete_change() {
        let (mut agent, db) = setup_agent().await;
        let temp_dir = TempDir::new().unwrap();
        agent.start_session().await.unwrap();

        let file_path = temp_dir.path().join("to_delete.txt");
        tokio::fs::write(&file_path, "content").await.unwrap();

        let change = RefactorChange::delete(
            agent.session().unwrap().id.clone(),
            "to_delete.txt".to_string(),
            "content".to_string(),
            "No longer needed".to_string(),
        );
        let change_id = change.id.clone();
        agent.add_proposed_change(change).await.unwrap();
        agent.approve_change(&change_id).await.unwrap();

        agent.apply_changes(temp_dir.path()).await.unwrap();
        assert!(!file_path.exists());
        db.cleanup().await;
    }

    #[tokio::test]
    async fn apply_delete_nonexistent_file_no_error() {
        let (mut agent, db) = setup_agent().await;
        let temp_dir = TempDir::new().unwrap();
        agent.start_session().await.unwrap();

        let change = RefactorChange::delete(
            agent.session().unwrap().id.clone(),
            "nonexistent.txt".to_string(),
            "".to_string(),
            "Already gone".to_string(),
        );
        let change_id = change.id.clone();
        agent.add_proposed_change(change).await.unwrap();
        agent.approve_change(&change_id).await.unwrap();

        let applied = agent.apply_changes(temp_dir.path()).await.unwrap();
        assert_eq!(applied, vec!["nonexistent.txt"]);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn apply_create_in_subdirectory() {
        let (mut agent, db) = setup_agent().await;
        let temp_dir = TempDir::new().unwrap();
        agent.start_session().await.unwrap();

        let change = RefactorChange::create(
            agent.session().unwrap().id.clone(),
            "sub/dir/file.txt".to_string(),
            "nested content".to_string(),
            "Create nested file".to_string(),
        );
        let change_id = change.id.clone();
        agent.add_proposed_change(change).await.unwrap();
        agent.approve_change(&change_id).await.unwrap();

        agent.apply_changes(temp_dir.path()).await.unwrap();

        let content = tokio::fs::read_to_string(temp_dir.path().join("sub/dir/file.txt"))
            .await
            .unwrap();
        assert_eq!(content, "nested content");
        db.cleanup().await;
    }

    #[tokio::test]
    async fn build_intent_prompt_with_empty_context() {
        let (agent, db) = setup_agent().await;
        let ctx = RefactorContext::new(ProductionMode::Running);
        let prompt = agent.build_intent_prompt("hello", &[], &ctx);
        assert!(!prompt.is_empty());
        db.cleanup().await;
    }

    #[tokio::test]
    async fn build_intent_prompt_with_work_status() {
        let (agent, db) = setup_agent().await;
        let mut ctx = RefactorContext::new(ProductionMode::Running);
        ctx.in_progress_work = vec!["task-1".to_string(), "task-2".to_string()];
        let prompt = agent.build_intent_prompt("change something", &[("user", "hi")], &ctx);
        assert!(!prompt.is_empty());
        db.cleanup().await;
    }

    #[tokio::test]
    async fn end_session_without_active_session() {
        let (mut agent, db) = setup_agent().await;
        // Should not error even without a session
        let result = agent.end_session().await;
        assert!(result.is_ok());
        db.cleanup().await;
    }

    #[test]
    fn confidence_serialization() {
        assert_eq!(serde_json::to_string(&Confidence::Low).unwrap(), "\"low\"");
        assert_eq!(
            serde_json::to_string(&Confidence::Medium).unwrap(),
            "\"medium\""
        );
        assert_eq!(
            serde_json::to_string(&Confidence::High).unwrap(),
            "\"high\""
        );
    }

    #[test]
    fn impact_severity_serialization() {
        assert_eq!(
            serde_json::to_string(&ImpactSeverity::None).unwrap(),
            "\"none\""
        );
        assert_eq!(
            serde_json::to_string(&ImpactSeverity::High).unwrap(),
            "\"high\""
        );
    }

    #[test]
    fn impact_recommendation_serialization() {
        assert_eq!(
            serde_json::to_string(&ImpactRecommendation::Continue).unwrap(),
            "\"continue\""
        );
        assert_eq!(
            serde_json::to_string(&ImpactRecommendation::HaltImmediately).unwrap(),
            "\"halt_immediately\""
        );
    }
}
