//! Planner Bot - Interactive PRD creation through conversation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use crate::llm::{LLMProvider, LLMRequest, Message};
use crate::types::{DataModelSketch, MilestoneSpec, PRDDocument, PRDStatus, TechnicalDecision};

/// Errors from the Planner Bot
#[derive(Error, Debug)]
pub enum PlannerBotError {
    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("session error: {0}")]
    SessionError(String),
}

/// Planning conversation phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlanningPhase {
    #[default]
    Discovery,
    Scoping,
    Technical,
    Milestones,
    Review,
}

impl std::fmt::Display for PlanningPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanningPhase::Discovery => write!(f, "Discovery"),
            PlanningPhase::Scoping => write!(f, "Scoping"),
            PlanningPhase::Technical => write!(f, "Technical"),
            PlanningPhase::Milestones => write!(f, "Milestones"),
            PlanningPhase::Review => write!(f, "Review"),
        }
    }
}

/// A message in the planning conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningMessage {
    pub role: PlanningMessageRole,
    pub content: String,
    pub phase: PlanningPhase,
    pub timestamp: DateTime<Utc>,
}

/// Role of a planning message sender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningMessageRole {
    User,
    Planner,
}

/// Tracks the state of a planning conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningSession {
    pub id: uuid::Uuid,
    pub phase: PlanningPhase,
    pub prd: PRDDocument,
    pub history: Vec<PlanningMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PlanningSession {
    /// Create a new planning session
    pub fn new(project_title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            phase: PlanningPhase::Discovery,
            prd: PRDDocument::new(project_title),
            history: vec![],
            created_at: now,
            updated_at: now,
        }
    }
}

/// The Planner Bot guides users through PRD creation
pub struct PlannerBot<P: LLMProvider> {
    provider: Arc<P>,
    model_id: String,
}

impl<P: LLMProvider> PlannerBot<P> {
    /// Create a new Planner Bot
    pub fn new(provider: Arc<P>, model_id: impl Into<String>) -> Self {
        Self {
            provider,
            model_id: model_id.into(),
        }
    }

    /// Start a new planning session
    pub fn start_session(&self, project_title: impl Into<String>) -> PlanningSession {
        PlanningSession::new(project_title)
    }

    /// Process a user message and return the planner's response
    pub async fn chat(
        &self,
        session: &mut PlanningSession,
        user_input: &str,
    ) -> Result<String, PlannerBotError> {
        // Add user message to history
        session.history.push(PlanningMessage {
            role: PlanningMessageRole::User,
            content: user_input.to_string(),
            phase: session.phase,
            timestamp: Utc::now(),
        });

        // Build LLM request
        let messages = self.build_messages(session);
        let system = self.system_prompt(session);

        let request = LLMRequest::new(&self.model_id, messages)
            .with_system(system)
            .with_max_tokens(4096);

        // Call LLM
        let response = self
            .provider
            .send_message(request)
            .await
            .map_err(|e| PlannerBotError::LlmError(e.to_string()))?;

        let content = response.content.clone();

        // Process response for structured updates and phase transitions
        self.process_response(session, &content);

        // Add planner response to history
        session.history.push(PlanningMessage {
            role: PlanningMessageRole::Planner,
            content: content.clone(),
            phase: session.phase,
            timestamp: Utc::now(),
        });

        session.updated_at = Utc::now();

        Ok(content)
    }

    /// Build conversation messages for the LLM
    fn build_messages(&self, session: &PlanningSession) -> Vec<Message> {
        session
            .history
            .iter()
            .map(|msg| match msg.role {
                PlanningMessageRole::User => Message::user(&msg.content),
                PlanningMessageRole::Planner => Message::assistant(&msg.content),
            })
            .collect()
    }

    /// Generate phase-specific system prompt
    fn system_prompt(&self, session: &PlanningSession) -> String {
        let phase = session.phase;
        let persona = r#"You are the Planner Bot, a methodical product planning assistant. You help users create Product Requirements Documents (PRDs) through structured conversation.

Personality traits:
- Methodical: Move through phases systematically
- Inquisitive: Ask clarifying questions rather than assuming
- Realistic: Push back on unrealistic scope or timelines
- Focused: Keep conversation on the current phase
- Concrete: Prefer specific details over vague descriptions

When you have enough information to update the PRD, include a JSON block in your response using ```json fences with the relevant fields."#;

        let phase_guidance = match phase {
            PlanningPhase::Discovery => r#"
CURRENT PHASE: Discovery
Goal: Understand the problem, users, and vision.
Ask about:
- What problem does this solve?
- Who are the target users?
- What is the high-level vision?
- What does success look like?

When you have a clear understanding, summarize and say "moving to scoping" to advance.
Include a JSON block to capture vision, problem_statement, and target_users when ready:
```json
{"vision": "...", "problem_statement": "...", "target_users": "..."}
```"#,
            PlanningPhase::Scoping => r#"
CURRENT PHASE: Scoping
Goal: Define boundaries and success criteria.
Ask about:
- What's in scope for v1 vs later?
- What are the success criteria?
- Any hard constraints (time, budget, team size)?

Push back on scope creep. When scoped, say "moving to technical" to advance.
Include a JSON block for success_criteria when ready:
```json
{"success_criteria": ["criterion 1", "criterion 2"]}
```"#,
            PlanningPhase::Technical => r#"
CURRENT PHASE: Technical
Goal: Make technology and architecture decisions.
Ask about:
- Language/framework choices
- Architecture patterns
- Data storage needs
- Integration points
- Key data models

When decisions are made, say "moving to milestones" to advance.
Include JSON blocks for decisions and data models:
```json
{"technical_decisions": [{"area": "...", "decision": "...", "rationale": "..."}]}
```
```json
{"data_models": [{"name": "...", "fields": ["..."], "description": "..."}]}
```"#,
            PlanningPhase::Milestones => r#"
CURRENT PHASE: Milestones
Goal: Break the project into deliverable phases.
Ask about:
- What should be built first?
- What depends on what?
- What's the MVP vs nice-to-have?

Each milestone should have a title, description, deliverables, and dependencies.
Include a JSON block when milestones are defined:
```json
{"milestones": [{"title": "...", "description": "...", "deliverables": ["..."], "dependencies": ["..."]}]}
```

When milestones are complete, say "moving to review" to advance."#,
            PlanningPhase::Review => r#"
CURRENT PHASE: Review
Goal: Final review of the complete PRD.
- Summarize the entire PRD
- Ask if anything needs changes
- Confirm the user is satisfied
- When approved, say "PRD approved" to finalize"#,
        };

        format!(
            "{}\n\nProject: {}\nScale: {}\n{}",
            persona,
            session.prd.title,
            session.prd.estimated_scale(),
            phase_guidance
        )
    }

    /// Process response for phase transitions and structured updates
    fn process_response(&self, session: &mut PlanningSession, response: &str) {
        // Extract and apply JSON blocks
        for json_str in extract_json_blocks(response) {
            apply_structured_update(&mut session.prd, &json_str);
        }

        // Detect phase transitions
        let lower = response.to_lowercase();
        let next_phase = match session.phase {
            PlanningPhase::Discovery if lower.contains("moving to scoping") => {
                Some(PlanningPhase::Scoping)
            }
            PlanningPhase::Scoping if lower.contains("moving to technical") => {
                Some(PlanningPhase::Technical)
            }
            PlanningPhase::Technical if lower.contains("moving to milestones") => {
                Some(PlanningPhase::Milestones)
            }
            PlanningPhase::Milestones if lower.contains("moving to review") => {
                Some(PlanningPhase::Review)
            }
            _ => None,
        };

        if let Some(phase) = next_phase {
            session.phase = phase;
        }
    }

    /// Finalize a PRD, marking it as approved
    pub fn finalize_prd(
        &self,
        session: &mut PlanningSession,
    ) -> Result<PRDDocument, PlannerBotError> {
        if !session.prd.is_complete() {
            return Err(PlannerBotError::SessionError(
                "PRD is incomplete: needs vision and at least one milestone".into(),
            ));
        }

        session.prd.status = PRDStatus::Approved;
        session.prd.updated_at = Utc::now();
        session.updated_at = Utc::now();

        Ok(session.prd.clone())
    }

    /// Export PRD as markdown
    pub fn export_markdown(&self, prd: &PRDDocument) -> String {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", prd.title));
        md.push_str(&format!("**Status:** {}\n", prd.status));
        md.push_str(&format!("**Scale:** {}\n\n", prd.estimated_scale()));

        if !prd.vision.is_empty() {
            md.push_str("## Vision\n\n");
            md.push_str(&prd.vision);
            md.push_str("\n\n");
        }

        if !prd.problem_statement.is_empty() {
            md.push_str("## Problem Statement\n\n");
            md.push_str(&prd.problem_statement);
            md.push_str("\n\n");
        }

        if !prd.target_users.is_empty() {
            md.push_str("## Target Users\n\n");
            md.push_str(&prd.target_users);
            md.push_str("\n\n");
        }

        if !prd.success_criteria.is_empty() {
            md.push_str("## Success Criteria\n\n");
            for criterion in &prd.success_criteria {
                md.push_str(&format!("- {}\n", criterion));
            }
            md.push('\n');
        }

        if !prd.technical_decisions.is_empty() {
            md.push_str("## Technical Decisions\n\n");
            md.push_str("| Area | Decision | Rationale |\n");
            md.push_str("|------|----------|----------|\n");
            for td in &prd.technical_decisions {
                md.push_str(&format!("| {} | {} | {} |\n", td.area, td.decision, td.rationale));
            }
            md.push('\n');
        }

        if !prd.data_models.is_empty() {
            md.push_str("## Data Models\n\n");
            for dm in &prd.data_models {
                md.push_str(&format!("### {}\n\n", dm.name));
                md.push_str(&format!("{}\n\n", dm.description));
                md.push_str("Fields:\n");
                for field in &dm.fields {
                    md.push_str(&format!("- {}\n", field));
                }
                md.push('\n');
            }
        }

        if !prd.milestones.is_empty() {
            md.push_str("## Milestones\n\n");
            for (i, ms) in prd.milestones.iter().enumerate() {
                md.push_str(&format!("### M{}: {}\n\n", i + 1, ms.title));
                md.push_str(&format!("{}\n\n", ms.description));
                if !ms.deliverables.is_empty() {
                    md.push_str("**Deliverables:**\n");
                    for d in &ms.deliverables {
                        md.push_str(&format!("- {}\n", d));
                    }
                    md.push('\n');
                }
                if !ms.dependencies.is_empty() {
                    md.push_str(&format!(
                        "**Dependencies:** {}\n\n",
                        ms.dependencies.join(", ")
                    ));
                }
            }
        }

        md
    }
}

/// Extract JSON code blocks from a response string
fn extract_json_blocks(text: &str) -> Vec<String> {
    let mut blocks = vec![];
    let mut remaining = text;

    while let Some(start) = remaining.find("```json") {
        let after_marker = &remaining[start + 7..];
        if let Some(end) = after_marker.find("```") {
            blocks.push(after_marker[..end].trim().to_string());
            remaining = &after_marker[end + 3..];
        } else {
            break;
        }
    }

    blocks
}

/// Apply a structured JSON update to a PRD document
fn apply_structured_update(prd: &mut PRDDocument, json_str: &str) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(v) = value.get("vision").and_then(|v| v.as_str()) {
            prd.vision = v.to_string();
        }
        if let Some(v) = value.get("problem_statement").and_then(|v| v.as_str()) {
            prd.problem_statement = v.to_string();
        }
        if let Some(v) = value.get("target_users").and_then(|v| v.as_str()) {
            prd.target_users = v.to_string();
        }
        if let Some(arr) = value.get("success_criteria") {
            if let Ok(criteria) = serde_json::from_value::<Vec<String>>(arr.clone()) {
                prd.success_criteria = criteria;
            }
        }
        if let Some(arr) = value.get("technical_decisions") {
            if let Ok(decisions) = serde_json::from_value::<Vec<TechnicalDecision>>(arr.clone()) {
                prd.technical_decisions = decisions;
            }
        }
        if let Some(arr) = value.get("data_models") {
            if let Ok(models) = serde_json::from_value::<Vec<DataModelSketch>>(arr.clone()) {
                prd.data_models = models;
            }
        }
        if let Some(arr) = value.get("milestones") {
            if let Ok(milestones) = serde_json::from_value::<Vec<MilestoneSpec>>(arr.clone()) {
                prd.milestones = milestones;
            }
        }
        prd.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planning_phase_default_is_discovery() {
        assert_eq!(PlanningPhase::default(), PlanningPhase::Discovery);
    }

    #[test]
    fn session_starts_in_discovery() {
        let session = PlanningSession::new("Test Project");
        assert_eq!(session.phase, PlanningPhase::Discovery);
        assert_eq!(session.prd.title, "Test Project");
        assert!(session.history.is_empty());
    }

    #[test]
    fn extract_json_blocks_finds_blocks() {
        let text = r#"Here is some text
```json
{"vision": "Build great things"}
```
And more text
```json
{"success_criteria": ["fast", "reliable"]}
```
Done."#;

        let blocks = extract_json_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains("vision"));
        assert!(blocks[1].contains("success_criteria"));
    }

    #[test]
    fn extract_json_blocks_empty_when_none() {
        let blocks = extract_json_blocks("No JSON here");
        assert!(blocks.is_empty());
    }

    #[test]
    fn apply_structured_update_sets_vision() {
        let mut prd = PRDDocument::new("Test");
        apply_structured_update(&mut prd, r#"{"vision": "Build something great"}"#);
        assert_eq!(prd.vision, "Build something great");
    }

    #[test]
    fn apply_structured_update_sets_milestones() {
        let mut prd = PRDDocument::new("Test");
        let json = r#"{"milestones": [{"title": "M1", "description": "First", "deliverables": ["API"], "dependencies": []}]}"#;
        apply_structured_update(&mut prd, json);
        assert_eq!(prd.milestones.len(), 1);
        assert_eq!(prd.milestones[0].title, "M1");
    }

    #[test]
    fn apply_structured_update_ignores_invalid_json() {
        let mut prd = PRDDocument::new("Test");
        apply_structured_update(&mut prd, "not json");
        assert!(prd.vision.is_empty());
    }

    #[test]
    fn apply_structured_update_sets_technical_decisions() {
        let mut prd = PRDDocument::new("Test");
        let json = r#"{"technical_decisions": [{"area": "Backend", "decision": "Rust", "rationale": "Speed"}]}"#;
        apply_structured_update(&mut prd, json);
        assert_eq!(prd.technical_decisions.len(), 1);
        assert_eq!(prd.technical_decisions[0].area, "Backend");
    }

    #[test]
    fn phase_transition_detection() {
        let mut session = PlanningSession::new("Test");
        assert_eq!(session.phase, PlanningPhase::Discovery);

        // Simulate response with phase transition
        let response = "Great summary! Moving to scoping phase now.";
        let lower = response.to_lowercase();
        if lower.contains("moving to scoping") {
            session.phase = PlanningPhase::Scoping;
        }
        assert_eq!(session.phase, PlanningPhase::Scoping);
    }

    #[test]
    fn finalize_fails_when_incomplete() {
        use crate::llm::{LLMError, LLMRequest, LLMResponse, StreamChunk};
        use async_trait::async_trait;
        use futures::Stream;
        use std::pin::Pin;

        struct MockProvider;

        #[async_trait]
        impl LLMProvider for MockProvider {
            async fn send_message(
                &self,
                _req: LLMRequest,
            ) -> Result<LLMResponse, LLMError> {
                unimplemented!()
            }
            async fn send_message_stream(
                &self,
                _req: LLMRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
            {
                unimplemented!()
            }
            fn provider_name(&self) -> &'static str {
                "mock"
            }
            fn model_id(&self) -> &str {
                "mock"
            }
        }

        let bot = PlannerBot::new(Arc::new(MockProvider), "mock");
        let mut session = bot.start_session("Test");

        let result = bot.finalize_prd(&mut session);
        assert!(result.is_err());
    }

    #[test]
    fn finalize_succeeds_when_complete() {
        use crate::llm::{LLMError, LLMRequest, LLMResponse, StreamChunk};
        use async_trait::async_trait;
        use futures::Stream;
        use std::pin::Pin;

        struct MockProvider;

        #[async_trait]
        impl LLMProvider for MockProvider {
            async fn send_message(
                &self,
                _req: LLMRequest,
            ) -> Result<LLMResponse, LLMError> {
                unimplemented!()
            }
            async fn send_message_stream(
                &self,
                _req: LLMRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
            {
                unimplemented!()
            }
            fn provider_name(&self) -> &'static str {
                "mock"
            }
            fn model_id(&self) -> &str {
                "mock"
            }
        }

        let bot = PlannerBot::new(Arc::new(MockProvider), "mock");
        let mut session = bot.start_session("Test");
        session.prd.vision = "A great product".into();
        session.prd.milestones.push(MilestoneSpec {
            title: "M1".into(),
            description: "First".into(),
            deliverables: vec![],
            dependencies: vec![],
        });

        let result = bot.finalize_prd(&mut session).unwrap();
        assert_eq!(result.status, PRDStatus::Approved);
    }

    #[test]
    fn export_markdown_includes_sections() {
        use crate::llm::{LLMError, LLMRequest, LLMResponse, StreamChunk};
        use async_trait::async_trait;
        use futures::Stream;
        use std::pin::Pin;

        struct MockProvider;

        #[async_trait]
        impl LLMProvider for MockProvider {
            async fn send_message(
                &self,
                _req: LLMRequest,
            ) -> Result<LLMResponse, LLMError> {
                unimplemented!()
            }
            async fn send_message_stream(
                &self,
                _req: LLMRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
            {
                unimplemented!()
            }
            fn provider_name(&self) -> &'static str {
                "mock"
            }
            fn model_id(&self) -> &str {
                "mock"
            }
        }

        let bot = PlannerBot::new(Arc::new(MockProvider), "mock");
        let mut prd = PRDDocument::new("Task Manager");
        prd.vision = "Manage tasks effectively".into();
        prd.problem_statement = "Tasks are hard to track".into();
        prd.success_criteria = vec!["Fast".into(), "Reliable".into()];
        prd.technical_decisions.push(TechnicalDecision {
            area: "Backend".into(),
            decision: "Rust".into(),
            rationale: "Performance".into(),
        });
        prd.milestones.push(MilestoneSpec {
            title: "Foundation".into(),
            description: "Core infrastructure".into(),
            deliverables: vec!["Database".into()],
            dependencies: vec![],
        });

        let md = bot.export_markdown(&prd);
        assert!(md.contains("# Task Manager"));
        assert!(md.contains("## Vision"));
        assert!(md.contains("## Problem Statement"));
        assert!(md.contains("## Success Criteria"));
        assert!(md.contains("## Technical Decisions"));
        assert!(md.contains("| Backend | Rust | Performance |"));
        assert!(md.contains("## Milestones"));
        assert!(md.contains("### M1: Foundation"));
    }
}
