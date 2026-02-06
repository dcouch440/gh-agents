//! CavernousStepStrategy — handles LLM interaction for cavernous routing phases.
//!
//! Phase 1 (SearchingConfigs): LLM generates a search query from the task description.
//! Phase 2 (SelectingConfig): LLM selects the best routing config from search results.
//!
//! The orchestrator (execute_cavernous_step in hub/dag) manages phase transitions
//! and non-LLM work (document search, config parsing, subtask execution).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::db::{AgentRow, DocumentSearchResult};
use crate::llm::{Message, TokenUsage, Tool};
use crate::types::RoutingAnalysis;

use super::super::error::HubError;
use super::super::strategy::ExecutionStrategy;

// ============================================================================
// Phase & State
// ============================================================================

/// Current phase of cavernous routing LLM interaction.
#[derive(Debug, Clone)]
enum CavernousPhase {
    /// Phase 1: LLM generates a search query for routing config documents.
    SearchingConfigs,
    /// Phase 2: LLM selects the best config from search results.
    SelectingConfig {
        search_results: Vec<DocumentSearchResult>,
    },
    /// All LLM phases complete.
    Done,
}

/// Internal mutable state shared between strategy methods via Arc<RwLock>.
struct CavernousState {
    phase: CavernousPhase,
    search_query: Option<String>,
    selected_document_id: Option<Uuid>,
    selection_reasoning: Option<String>,
}

// ============================================================================
// Config
// ============================================================================

/// Configuration for a cavernous step's LLM phases.
pub struct CavernousStepConfig {
    /// The agent assigned to this cavernous step.
    pub agent: AgentRow,
    /// The composed task prompt (variable-resolved, with port inputs).
    pub user_prompt: String,
    /// User ID for this execution.
    pub user_id: Uuid,
    /// Pipeline run ID.
    pub run_id: Uuid,
}

// ============================================================================
// Strategy
// ============================================================================

/// Strategy for cavernous routing LLM phases (search query generation, config selection).
///
/// Uses `Arc<RwLock<CavernousState>>` for interior mutability — `on_complete()` receives
/// `&self` but needs to store parsed results for the orchestrator to read.
pub struct CavernousStepStrategy {
    config: CavernousStepConfig,
    state_inner: Arc<RwLock<CavernousState>>,
}

impl CavernousStepStrategy {
    pub fn new(config: CavernousStepConfig) -> Self {
        Self {
            config,
            state_inner: Arc::new(RwLock::new(CavernousState {
                phase: CavernousPhase::SearchingConfigs,
                search_query: None,
                selected_document_id: None,
                selection_reasoning: None,
            })),
        }
    }

    /// Transition to SelectingConfig phase with document search results.
    /// Called by the orchestrator after programmatic document search.
    pub async fn set_search_results(&self, results: Vec<DocumentSearchResult>) {
        let mut state = self.state_inner.write().await;
        state.phase = CavernousPhase::SelectingConfig {
            search_results: results,
        };
    }

    /// Read the generated search query (populated after Phase 1 completes).
    pub async fn search_query(&self) -> Option<String> {
        self.state_inner.read().await.search_query.clone()
    }

    /// Read the selected document ID (populated after Phase 2 completes).
    pub async fn selected_document_id(&self) -> Option<Uuid> {
        self.state_inner.read().await.selected_document_id
    }

    /// Read the selection reasoning (populated after Phase 2 completes).
    pub async fn selection_reasoning(&self) -> Option<String> {
        self.state_inner.read().await.selection_reasoning.clone()
    }

    /// Build a RoutingAnalysis for storage in agent_executions.
    pub async fn build_routing_analysis(
        &self,
        documents_found: Vec<crate::types::DocumentSummary>,
    ) -> Option<RoutingAnalysis> {
        let state = self.state_inner.read().await;
        let selected_id = state.selected_document_id?;
        let query = state.search_query.clone()?;
        let reasoning = state.selection_reasoning.clone().unwrap_or_default();
        Some(RoutingAnalysis {
            search_query: query,
            documents_found,
            selected_document_id: selected_id,
            reasoning,
            collaborative_selection: false,
        })
    }
}

// ============================================================================
// ExecutionStrategy implementation
// ============================================================================

#[async_trait]
impl ExecutionStrategy for CavernousStepStrategy {
    fn system_prompt(&self) -> &str {
        // System prompt varies by phase — build_messages handles per-phase prompting.
        // The engine prepends this as a system message.
        "You are a routing configuration analyst for a workflow execution system."
    }

    fn tools(&self) -> Vec<Tool> {
        vec![] // No tools for search/select phases
    }

    fn model_id(&self) -> &str {
        &self.config.agent.model_id
    }

    fn max_rounds(&self) -> u32 {
        1 // Single-turn for each phase
    }

    fn context_budget(&self) -> usize {
        100_000 // Routing prompts are small
    }

    fn streaming(&self) -> bool {
        false
    }

    fn temperature(&self) -> f32 {
        0.2 // Low temperature for deterministic selection
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        let state = self.state_inner.read().await;

        match &state.phase {
            CavernousPhase::SearchingConfigs => {
                let prompt = format!(
                    "Analyze the following task and generate a concise search query \
                     (3-8 words) to find the best routing configuration document.\n\n\
                     Task:\n{}\n\n\
                     Respond with ONLY the search query, nothing else.",
                    self.config.user_prompt
                );
                Ok(vec![Message::user(&prompt)])
            }
            CavernousPhase::SelectingConfig { search_results } => {
                let options_text: String = search_results
                    .iter()
                    .enumerate()
                    .map(|(i, doc)| {
                        format!(
                            "{}. **{}**\n   Summary: {}\n   Snippet: {}",
                            i,
                            doc.title,
                            doc.summary.as_deref().unwrap_or("N/A"),
                            doc.snippet
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");

                let prompt = format!(
                    "Given the following task and available routing configurations, \
                     select the best matching config.\n\n\
                     Task:\n{}\n\n\
                     Available configurations:\n{}\n\n\
                     Respond with JSON only:\n\
                     {{\"selected_index\": <number>, \"reasoning\": \"<brief explanation>\"}}",
                    self.config.user_prompt, options_text
                );
                Ok(vec![Message::user(&prompt)])
            }
            CavernousPhase::Done => Ok(vec![]),
        }
    }

    async fn execute_tool(&self, _name: &str, _input: &Value) -> Value {
        serde_json::json!({"error": "cavernous strategy does not execute tools"})
    }

    async fn on_complete(&self, response: &str, _usage: &TokenUsage) -> Result<(), HubError> {
        let mut state = self.state_inner.write().await;

        match &state.phase {
            CavernousPhase::SearchingConfigs => {
                // Response is the search query text
                let query = response.trim().to_string();
                state.search_query = Some(query);
            }
            CavernousPhase::SelectingConfig { search_results } => {
                // Parse JSON selection response
                let selection = parse_selection_response(response, search_results.len())?;
                state.selected_document_id = Some(search_results[selection.selected_index].id);
                state.selection_reasoning = Some(selection.reasoning);
                state.phase = CavernousPhase::Done;
            }
            CavernousPhase::Done => {}
        }

        Ok(())
    }
}

// ============================================================================
// Response Parsing
// ============================================================================

/// Parsed config selection from LLM response.
pub(crate) struct SelectionResponse {
    pub selected_index: usize,
    pub reasoning: String,
}

/// Parse the LLM's config selection JSON response.
pub(crate) fn parse_selection_response(
    response: &str,
    num_options: usize,
) -> Result<SelectionResponse, HubError> {
    let trimmed = response.trim();

    // Try direct JSON parse
    let parsed: Value = serde_json::from_str(trimmed)
        .or_else(|_| {
            // Try extracting from code fence
            if let Some(start) = trimmed.find('{') {
                if let Some(end) = trimmed.rfind('}') {
                    return serde_json::from_str(&trimmed[start..=end]);
                }
            }
            Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "no JSON found in response",
            )))
        })
        .map_err(|e| {
            HubError::Internal(anyhow::anyhow!(
                "Failed to parse config selection response: {}. Response: {}",
                e,
                trimmed
            ))
        })?;

    let selected_index = parsed["selected_index"].as_u64().ok_or_else(|| {
        HubError::Internal(anyhow::anyhow!(
            "Missing or invalid 'selected_index' in selection response"
        ))
    })? as usize;

    if selected_index >= num_options {
        return Err(HubError::Internal(anyhow::anyhow!(
            "selected_index {} out of range (0..{})",
            selected_index,
            num_options
        )));
    }

    let reasoning = parsed["reasoning"].as_str().unwrap_or("").to_string();

    Ok(SelectionResponse {
        selected_index,
        reasoning,
    })
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
