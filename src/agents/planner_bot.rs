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
            .with_max_tokens(8192);

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
            PlanningPhase::Discovery => {
                r#"
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
```"#
            }
            PlanningPhase::Scoping => {
                r#"
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
```"#
            }
            PlanningPhase::Technical => {
                r#"
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
```"#
            }
            PlanningPhase::Milestones => {
                r#"
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

When milestones are complete, say "moving to review" to advance."#
            }
            PlanningPhase::Review => {
                r#"
CURRENT PHASE: Review
Goal: Final review of the complete PRD.
- Summarize the entire PRD
- Ask if anything needs changes
- Confirm the user is satisfied
- When approved, say "PRD approved" to finalize"#
            }
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
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    td.area, td.decision, td.rationale
                ));
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
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
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
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
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
    fn planning_phase_display_all() {
        assert_eq!(PlanningPhase::Discovery.to_string(), "Discovery");
        assert_eq!(PlanningPhase::Scoping.to_string(), "Scoping");
        assert_eq!(PlanningPhase::Technical.to_string(), "Technical");
        assert_eq!(PlanningPhase::Milestones.to_string(), "Milestones");
        assert_eq!(PlanningPhase::Review.to_string(), "Review");
    }

    #[test]
    fn planning_phase_serde_roundtrip() {
        let variants = [
            PlanningPhase::Discovery,
            PlanningPhase::Scoping,
            PlanningPhase::Technical,
            PlanningPhase::Milestones,
            PlanningPhase::Review,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let parsed: PlanningPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, parsed);
        }
    }

    #[test]
    fn planning_message_role_serde_roundtrip() {
        let roles = [PlanningMessageRole::User, PlanningMessageRole::Planner];
        for r in &roles {
            let json = serde_json::to_string(r).unwrap();
            let parsed: PlanningMessageRole = serde_json::from_str(&json).unwrap();
            assert_eq!(*r, parsed);
        }
    }

    #[test]
    fn planner_bot_error_display() {
        assert!(PlannerBotError::LlmError("timeout".into())
            .to_string()
            .contains("timeout"));
        assert!(PlannerBotError::ParseError("bad json".into())
            .to_string()
            .contains("bad json"));
        assert!(PlannerBotError::SessionError("expired".into())
            .to_string()
            .contains("expired"));
    }

    #[test]
    fn planning_session_new_defaults() {
        let session = PlanningSession::new("My Project");
        assert_eq!(session.prd.title, "My Project");
        assert_eq!(session.phase, PlanningPhase::Discovery);
        assert!(session.history.is_empty());
        assert!(session.created_at <= session.updated_at);
    }

    #[test]
    fn extract_json_blocks_unclosed() {
        let text = "```json\n{\"a\": 1}\nno closing fence";
        let blocks = extract_json_blocks(text);
        assert!(blocks.is_empty());
    }

    #[test]
    fn apply_structured_update_problem_statement() {
        let mut prd = PRDDocument::new("Test");
        apply_structured_update(&mut prd, r#"{"problem_statement": "Users need X"}"#);
        assert_eq!(prd.problem_statement, "Users need X");
    }

    #[test]
    fn apply_structured_update_target_users() {
        let mut prd = PRDDocument::new("Test");
        apply_structured_update(&mut prd, r#"{"target_users": "Developers"}"#);
        assert_eq!(prd.target_users, "Developers");
    }

    #[test]
    fn apply_structured_update_success_criteria() {
        let mut prd = PRDDocument::new("Test");
        apply_structured_update(&mut prd, r#"{"success_criteria": ["fast", "reliable"]}"#);
        assert_eq!(prd.success_criteria, vec!["fast", "reliable"]);
    }

    #[test]
    fn apply_structured_update_data_models() {
        let mut prd = PRDDocument::new("Test");
        let json = r#"{"data_models": [{"name": "User", "fields": ["id", "name"], "description": "A user"}]}"#;
        apply_structured_update(&mut prd, json);
        assert_eq!(prd.data_models.len(), 1);
        assert_eq!(prd.data_models[0].name, "User");
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
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
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

    // Helper to create a PlannerBot with a mock provider
    fn mock_bot() -> PlannerBot<impl LLMProvider> {
        use crate::llm::{LLMError, LLMRequest, LLMResponse, StreamChunk};
        use async_trait::async_trait;
        use futures::Stream;
        use std::pin::Pin;

        struct MockProvider;

        #[async_trait]
        impl LLMProvider for MockProvider {
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
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

        PlannerBot::new(Arc::new(MockProvider), "mock")
    }

    #[test]
    fn system_prompt_discovery_phase() {
        let bot = mock_bot();
        let session = bot.start_session("My App");
        let prompt = bot.system_prompt(&session);
        assert!(prompt.contains("CURRENT PHASE: Discovery"));
        assert!(prompt.contains("target users"));
        assert!(prompt.contains("vision"));
        assert!(prompt.contains("problem_statement"));
        assert!(prompt.contains("My App"));
    }

    #[test]
    fn system_prompt_scoping_phase() {
        let bot = mock_bot();
        let mut session = bot.start_session("My App");
        session.phase = PlanningPhase::Scoping;
        let prompt = bot.system_prompt(&session);
        assert!(prompt.contains("CURRENT PHASE: Scoping"));
        assert!(prompt.contains("success_criteria"));
        assert!(prompt.contains("scope"));
    }

    #[test]
    fn system_prompt_technical_phase() {
        let bot = mock_bot();
        let mut session = bot.start_session("My App");
        session.phase = PlanningPhase::Technical;
        let prompt = bot.system_prompt(&session);
        assert!(prompt.contains("CURRENT PHASE: Technical"));
        assert!(prompt.contains("technical_decisions"));
        assert!(prompt.contains("data_models"));
    }

    #[test]
    fn system_prompt_milestones_phase() {
        let bot = mock_bot();
        let mut session = bot.start_session("My App");
        session.phase = PlanningPhase::Milestones;
        let prompt = bot.system_prompt(&session);
        assert!(prompt.contains("CURRENT PHASE: Milestones"));
        assert!(prompt.contains("milestones"));
        assert!(prompt.contains("deliverables"));
    }

    #[test]
    fn system_prompt_review_phase() {
        let bot = mock_bot();
        let mut session = bot.start_session("My App");
        session.phase = PlanningPhase::Review;
        let prompt = bot.system_prompt(&session);
        assert!(prompt.contains("CURRENT PHASE: Review"));
        assert!(prompt.contains("PRD approved"));
    }

    #[test]
    fn process_response_discovery_to_scoping() {
        let bot = mock_bot();
        let mut session = bot.start_session("Test");
        assert_eq!(session.phase, PlanningPhase::Discovery);
        bot.process_response(&mut session, "Great! Moving to scoping now.");
        assert_eq!(session.phase, PlanningPhase::Scoping);
    }

    #[test]
    fn process_response_scoping_to_technical() {
        let bot = mock_bot();
        let mut session = bot.start_session("Test");
        session.phase = PlanningPhase::Scoping;
        bot.process_response(&mut session, "Scope defined. Moving to technical.");
        assert_eq!(session.phase, PlanningPhase::Technical);
    }

    #[test]
    fn process_response_technical_to_milestones() {
        let bot = mock_bot();
        let mut session = bot.start_session("Test");
        session.phase = PlanningPhase::Technical;
        bot.process_response(&mut session, "Decisions made. Moving to milestones.");
        assert_eq!(session.phase, PlanningPhase::Milestones);
    }

    #[test]
    fn process_response_milestones_to_review() {
        let bot = mock_bot();
        let mut session = bot.start_session("Test");
        session.phase = PlanningPhase::Milestones;
        bot.process_response(&mut session, "All set. Moving to review.");
        assert_eq!(session.phase, PlanningPhase::Review);
    }

    #[test]
    fn process_response_no_transition_on_wrong_phase() {
        let bot = mock_bot();
        let mut session = bot.start_session("Test");
        // Discovery phase should not respond to "moving to technical"
        bot.process_response(&mut session, "Moving to technical.");
        assert_eq!(session.phase, PlanningPhase::Discovery);
    }

    #[test]
    fn process_response_extracts_json_and_updates_prd() {
        let bot = mock_bot();
        let mut session = bot.start_session("Test");
        let response = "Here is the vision:\n```json\n{\"vision\": \"Build X\", \"target_users\": \"Devs\"}\n```\nMoving to scoping.";
        bot.process_response(&mut session, response);
        assert_eq!(session.prd.vision, "Build X");
        assert_eq!(session.prd.target_users, "Devs");
        assert_eq!(session.phase, PlanningPhase::Scoping);
    }

    #[test]
    fn build_messages_maps_roles_correctly() {
        let bot = mock_bot();
        let mut session = bot.start_session("Test");
        session.history.push(PlanningMessage {
            role: PlanningMessageRole::User,
            content: "Hello".into(),
            phase: PlanningPhase::Discovery,
            timestamp: Utc::now(),
        });
        session.history.push(PlanningMessage {
            role: PlanningMessageRole::Planner,
            content: "Hi there".into(),
            phase: PlanningPhase::Discovery,
            timestamp: Utc::now(),
        });
        session.history.push(PlanningMessage {
            role: PlanningMessageRole::User,
            content: "More info".into(),
            phase: PlanningPhase::Discovery,
            timestamp: Utc::now(),
        });

        let messages = bot.build_messages(&session);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, crate::llm::Role::User);
        assert_eq!(messages[1].role, crate::llm::Role::Assistant);
        assert_eq!(messages[2].role, crate::llm::Role::User);
        assert_eq!(messages[0].text(), "Hello");
        assert_eq!(messages[1].text(), "Hi there");
    }

    #[test]
    fn build_messages_empty_history() {
        let bot = mock_bot();
        let session = bot.start_session("Test");
        let messages = bot.build_messages(&session);
        assert!(messages.is_empty());
    }

    #[test]
    fn export_markdown_with_data_models() {
        let bot = mock_bot();
        let mut prd = PRDDocument::new("DataApp");
        prd.data_models.push(DataModelSketch {
            name: "Task".into(),
            fields: vec!["id: UUID".into(), "title: String".into()],
            description: "A work item".into(),
        });

        let md = bot.export_markdown(&prd);
        assert!(md.contains("## Data Models"));
        assert!(md.contains("### Task"));
        assert!(md.contains("A work item"));
        assert!(md.contains("- id: UUID"));
        assert!(md.contains("- title: String"));
    }

    #[test]
    fn export_markdown_with_target_users() {
        let bot = mock_bot();
        let mut prd = PRDDocument::new("UserApp");
        prd.target_users = "Enterprise developers".into();

        let md = bot.export_markdown(&prd);
        assert!(md.contains("## Target Users"));
        assert!(md.contains("Enterprise developers"));
    }

    #[test]
    fn export_markdown_with_milestone_dependencies() {
        let bot = mock_bot();
        let mut prd = PRDDocument::new("DepApp");
        prd.milestones.push(MilestoneSpec {
            title: "M2".into(),
            description: "Second phase".into(),
            deliverables: vec!["API".into(), "Docs".into()],
            dependencies: vec!["M1".into()],
        });

        let md = bot.export_markdown(&prd);
        assert!(md.contains("**Dependencies:** M1"));
        assert!(md.contains("- API"));
        assert!(md.contains("- Docs"));
    }

    #[test]
    fn export_markdown_empty_prd_has_title_and_status() {
        let bot = mock_bot();
        let prd = PRDDocument::new("EmptyProject");
        let md = bot.export_markdown(&prd);
        assert!(md.contains("# EmptyProject"));
        assert!(md.contains("**Status:**"));
        assert!(md.contains("**Scale:**"));
        // Should not contain optional sections
        assert!(!md.contains("## Vision"));
        assert!(!md.contains("## Data Models"));
    }

    #[tokio::test]
    async fn chat_sends_message_and_processes_response() {
        use crate::llm::{LLMError, LLMRequest, LLMResponse, StreamChunk};
        use async_trait::async_trait;
        use futures::Stream;
        use std::pin::Pin;

        struct MockChatProvider;

        #[async_trait]
        impl LLMProvider for MockChatProvider {
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
                Ok(LLMResponse {
                    content: "Great vision! ```json\n{\"vision\": \"Test vision\"}\n``` Moving to scoping.".to_string(),
                    content_blocks: vec![],
                    usage: crate::llm::TokenUsage { input_tokens: 10, output_tokens: 20 },
                    model: "mock".to_string(),
                    stop_reason: crate::llm::StopReason::EndTurn,
                })
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

        let bot = PlannerBot::new(Arc::new(MockChatProvider), "mock");
        let mut session = bot.start_session("Chat Test");
        assert_eq!(session.phase, PlanningPhase::Discovery);

        let response = bot.chat(&mut session, "Here is my idea").await.unwrap();
        assert!(response.contains("Moving to scoping"));
        assert_eq!(session.phase, PlanningPhase::Scoping);
        assert_eq!(session.prd.vision, "Test vision");
        // History should have user + planner messages
        assert_eq!(session.history.len(), 2);
        assert_eq!(session.history[0].role, PlanningMessageRole::User);
        assert_eq!(session.history[0].content, "Here is my idea");
        assert_eq!(session.history[1].role, PlanningMessageRole::Planner);
    }

    #[tokio::test]
    async fn chat_returns_error_on_llm_failure() {
        use crate::llm::{LLMError, LLMRequest, LLMResponse, StreamChunk};
        use async_trait::async_trait;
        use futures::Stream;
        use std::pin::Pin;

        struct FailProvider;

        #[async_trait]
        impl LLMProvider for FailProvider {
            async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
                Err(LLMError::AuthError("connection refused".into()))
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

        let bot = PlannerBot::new(Arc::new(FailProvider), "mock");
        let mut session = bot.start_session("Fail Test");

        let result = bot.chat(&mut session, "hello").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            PlannerBotError::LlmError(msg) => assert!(msg.contains("connection refused")),
            other => panic!("expected LlmError, got: {:?}", other),
        }
        // User message was added but no planner message
        assert_eq!(session.history.len(), 1);
    }

    #[test]
    fn apply_structured_update_ignores_invalid_typed_arrays() {
        let mut prd = PRDDocument::new("Test");
        // success_criteria expects Vec<String> but we give it an object
        apply_structured_update(&mut prd, r#"{"success_criteria": "not an array"}"#);
        assert!(prd.success_criteria.is_empty());

        // technical_decisions expects specific structure
        apply_structured_update(
            &mut prd,
            r#"{"technical_decisions": [{"wrong": "fields"}]}"#,
        );
        assert!(prd.technical_decisions.is_empty());

        // data_models expects specific structure
        apply_structured_update(&mut prd, r#"{"data_models": "bad"}"#);
        assert!(prd.data_models.is_empty());

        // milestones expects specific structure
        apply_structured_update(&mut prd, r#"{"milestones": [123]}"#);
        assert!(prd.milestones.is_empty());
    }

    #[test]
    fn apply_structured_update_multiple_fields_at_once() {
        let mut prd = PRDDocument::new("Test");
        let json = r#"{
            "vision": "V",
            "problem_statement": "P",
            "target_users": "T",
            "success_criteria": ["A", "B"],
            "technical_decisions": [{"area": "X", "decision": "Y", "rationale": "Z"}],
            "data_models": [{"name": "M", "fields": ["f1"], "description": "D"}],
            "milestones": [{"title": "M1", "description": "D1", "deliverables": ["d"], "dependencies": []}]
        }"#;
        apply_structured_update(&mut prd, json);
        assert_eq!(prd.vision, "V");
        assert_eq!(prd.problem_statement, "P");
        assert_eq!(prd.target_users, "T");
        assert_eq!(prd.success_criteria.len(), 2);
        assert_eq!(prd.technical_decisions.len(), 1);
        assert_eq!(prd.data_models.len(), 1);
        assert_eq!(prd.milestones.len(), 1);
    }

    #[test]
    fn extract_json_blocks_multiple() {
        let text = "Text\n```json\n{\"a\": 1}\n```\nMiddle\n```json\n{\"b\": 2}\n```\nEnd\n```json\n{\"c\": 3}\n```";
        let blocks = extract_json_blocks(text);
        assert_eq!(blocks.len(), 3);
    }

    #[test]
    fn process_response_no_transition_in_review() {
        let bot = mock_bot();
        let mut session = bot.start_session("Test");
        session.phase = PlanningPhase::Review;
        bot.process_response(&mut session, "Everything looks good.");
        assert_eq!(session.phase, PlanningPhase::Review);
    }

    #[test]
    fn export_markdown_milestone_empty_deliverables_and_deps() {
        let bot = mock_bot();
        let mut prd = PRDDocument::new("Minimal");
        prd.milestones.push(MilestoneSpec {
            title: "M1".into(),
            description: "Basic".into(),
            deliverables: vec![],
            dependencies: vec![],
        });

        let md = bot.export_markdown(&prd);
        assert!(md.contains("### M1: M1"));
        assert!(!md.contains("**Deliverables:**"));
        assert!(!md.contains("**Dependencies:**"));
    }

    #[test]
    fn finalize_prd_sets_timestamps() {
        let bot = mock_bot();
        let mut session = bot.start_session("Test");
        session.prd.vision = "Vision".into();
        session.prd.milestones.push(MilestoneSpec {
            title: "M1".into(),
            description: "D".into(),
            deliverables: vec![],
            dependencies: vec![],
        });
        let before = Utc::now();
        let prd = bot.finalize_prd(&mut session).unwrap();
        assert!(prd.updated_at >= before);
        assert!(session.updated_at >= before);
    }

    #[test]
    fn planning_session_serde_roundtrip() {
        let mut session = PlanningSession::new("Serde Test");
        session.history.push(PlanningMessage {
            role: PlanningMessageRole::User,
            content: "hello".into(),
            phase: PlanningPhase::Discovery,
            timestamp: Utc::now(),
        });
        let json = serde_json::to_string(&session).unwrap();
        let parsed: PlanningSession = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.prd.title, "Serde Test");
        assert_eq!(parsed.history.len(), 1);
        assert_eq!(parsed.phase, PlanningPhase::Discovery);
    }

    #[test]
    fn system_prompt_contains_persona() {
        let bot = mock_bot();
        let session = bot.start_session("Test");
        let prompt = bot.system_prompt(&session);
        assert!(prompt.contains("Planner Bot"));
        assert!(prompt.contains("Methodical"));
        assert!(prompt.contains("Inquisitive"));
    }
}
