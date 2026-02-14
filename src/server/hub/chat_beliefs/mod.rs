//! Chat belief extraction and formatting for neighbor awareness.
//!
//! After each chat on a node, Haiku extracts beliefs from the conversation.
//! When building a system prompt for a connected node, those beliefs are
//! pulled from the DB and formatted for injection into `<board_context>`.

use std::collections::BTreeMap;

use serde::Deserialize;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::protocols::{roles, vars};
use crate::db::{BeliefRow, ChatMessageRow};
use crate::llm::{
    AnthropicClient, AnthropicConfig, LLMProvider, LLMRequest, Message as LlmMessage,
};
use crate::server::hub::protocols::json_utils::parse_structured_output;
use crate::server::state::AppState;

mod tests;

/// Max tokens for the belief extraction response.
const MAX_TOKENS_EXTRACTION: u32 = 4096;

/// Maximum number of messages to load for extraction.
const MAX_CONVERSATION_MESSAGES: u32 = 100;

/// Minimum messages before attempting extraction.
const MIN_MESSAGES_FOR_EXTRACTION: u32 = 2;

// ── Spawn helper ────────────────────────────────────────────────────────

/// Spawn a background belief extraction for a step's chat conversation.
/// Non-blocking — fires and forgets. Errors are logged, not propagated.
pub fn spawn_chat_belief_extraction(
    state: AppState,
    workflow_id: Uuid,
    step_id: Uuid,
    session_id: Uuid,
) {
    tokio::spawn(async move {
        if let Err(e) = extract_and_replace_beliefs(&state, workflow_id, step_id, session_id).await
        {
            tracing::error!("Chat belief extraction failed for step {step_id}: {e}");
        }
    });
}

// ── Extraction ──────────────────────────────────────────────────────────

/// Load conversation, extract beliefs via Haiku, replace in DB.
async fn extract_and_replace_beliefs(
    state: &AppState,
    workflow_id: Uuid,
    step_id: Uuid,
    session_id: Uuid,
) -> Result<(), anyhow::Error> {
    // 1. Load step info
    let step = state
        .repos()
        .workflows
        .get_step(step_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Step {step_id} not found"))?;

    let node_name = step.name.as_deref().unwrap_or("(unnamed)");

    // 2. Load existing beliefs from connected nodes for board awareness
    let connected_beliefs = state
        .repos()
        .workflows
        .get_beliefs_for_connected_steps(workflow_id, step_id)
        .await
        .unwrap_or_default();

    let board_beliefs_text = format_beliefs_for_extraction(&connected_beliefs);

    // 3. Check message count
    let msg_count = state.repo().count_session_messages(session_id).await?;

    if msg_count < MIN_MESSAGES_FOR_EXTRACTION {
        return Ok(());
    }

    // 4. Load conversation
    let messages = state
        .repo()
        .get_session_history(session_id, MAX_CONVERSATION_MESSAGES)
        .await?;

    if messages.is_empty() {
        return Ok(());
    }

    let conversation = format_conversation(&messages);

    // 5. Resolve protocol
    let mut vars_map = std::collections::HashMap::new();
    vars_map.insert(
        vars::chat_belief::NODE_NAME.to_string(),
        node_name.to_string(),
    );
    vars_map.insert(
        vars::chat_belief::NODE_ARCHETYPE.to_string(),
        step.execution_mode.clone(),
    );
    vars_map.insert(vars::chat_belief::CONVERSATION.to_string(), conversation);
    vars_map.insert(
        vars::chat_belief::BOARD_BELIEFS.to_string(),
        board_beliefs_text,
    );

    let resolved = roles::CHAT_BELIEF_EXTRACTOR.resolve(&vars_map);

    // 6. Call Haiku
    let config = AnthropicConfig::from_env()?;
    let client = AnthropicClient::new(config)?;

    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(resolved.user_prompt)],
    )
    .with_system(&resolved.system_prompt)
    .with_max_tokens(MAX_TOKENS_EXTRACTION);

    let response = client.send_message(request).await?;

    // 7. Parse beliefs
    let extracted = parse_extraction_output(&response.content);

    info!(
        step_id = %step_id,
        beliefs = extracted.len(),
        "Extracted chat beliefs"
    );

    // 8. Convert to BeliefRows
    let belief_rows: Vec<BeliefRow> = extracted
        .into_iter()
        .map(|b| BeliefRow {
            id: Uuid::new_v4(),
            workflow_id,
            workflow_execution_id: None,
            source_step_id: step_id,
            source_document_title: None,
            source_document_def_id: None,
            source_phase: "chat".to_string(),
            content: b.content,
            reasoning: b.reasoning,
            belief_type: b.belief_type,
            confidence: b.confidence,
            confidence_justification: b.confidence_justification,
            semantic_tags: b.semantic_tags,
            emotional_tone: b.emotional_tone,
            cross_source_tension: b.cross_source_tension,
            source_step_name: node_name.to_string(),
            extraction_model: crate::constants::MODEL_HAIKU.to_string(),
            extraction_tokens_in: response.usage.input_tokens as i32,
            extraction_tokens_out: response.usage.output_tokens as i32,
            created_at: chrono::Utc::now(),
        })
        .collect();

    // 9. Replace in DB
    state
        .repos()
        .workflows
        .replace_chat_beliefs(step_id, &belief_rows)
        .await?;

    Ok(())
}

fn format_conversation(messages: &[ChatMessageRow]) -> String {
    messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

// ── Parsing ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct BeliefExtractionOutput {
    beliefs: Vec<ExtractedBelief>,
}

#[derive(Debug, Deserialize)]
struct ExtractedBelief {
    content: String,
    reasoning: String,
    belief_type: String,
    confidence: String,
    #[serde(default)]
    confidence_justification: Option<String>,
    #[serde(default)]
    semantic_tags: Vec<String>,
    #[serde(default)]
    emotional_tone: Option<String>,
    #[serde(default)]
    cross_source_tension: Option<String>,
}

pub(crate) fn parse_extraction_output(content: &str) -> Vec<ExtractedBelief> {
    match parse_structured_output(content) {
        Some(json) => match serde_json::from_value::<BeliefExtractionOutput>(json) {
            Ok(output) => output.beliefs,
            Err(e) => {
                warn!("Failed to deserialize chat belief extraction output: {}", e);
                vec![]
            }
        },
        None => {
            warn!("No structured JSON found in chat belief extraction response");
            vec![]
        }
    }
}

// ── Formatting ──────────────────────────────────────────────────────────

/// Format connected-node beliefs as compact context for the extraction prompt.
///
/// Passed to Haiku so it can see what the rest of the board already knows
/// and produce grounded, non-redundant beliefs.
pub(crate) fn format_beliefs_for_extraction(beliefs: &[BeliefRow]) -> String {
    if beliefs.is_empty() {
        return "No beliefs from other nodes yet.".to_string();
    }

    let mut by_node: BTreeMap<&str, Vec<&BeliefRow>> = BTreeMap::new();
    for belief in beliefs {
        by_node
            .entry(&belief.source_step_name)
            .or_default()
            .push(belief);
    }

    let mut out = String::new();
    for (i, (node_name, node_beliefs)) in by_node.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!("[{}]\n", node_name));
        for belief in node_beliefs {
            out.push_str(&format!("- {} ({})\n", belief.content, belief.belief_type));
        }
    }

    out
}

/// Format connected-node beliefs into readable text for system prompt injection.
///
/// Groups beliefs by source node name, then lists each belief as a plain
/// sentence with `[type, confidence]` inline tags. Reads like notes from
/// a colleague, not a structured spec.
///
/// Beliefs with `cross_source_tension` starting with "SUPERSEDED:" indicate
/// a correction — they stay visible, and the tension note is appended so the
/// consuming agent understands the pivot.
pub fn format_beliefs_as_board_context(beliefs: &[BeliefRow]) -> String {
    if beliefs.is_empty() {
        return "No neighboring nodes have active conversations yet.".to_string();
    }

    // Group by source node (BTreeMap for deterministic ordering)
    let mut by_node: BTreeMap<&str, Vec<&BeliefRow>> = BTreeMap::new();
    for belief in beliefs {
        by_node
            .entry(&belief.source_step_name)
            .or_default()
            .push(belief);
    }

    let mut out = String::new();

    for (i, (node_name, node_beliefs)) in by_node.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }

        out.push_str(&format!("{}:\n", node_name));

        for belief in node_beliefs {
            let confidence_tag = match belief.confidence.as_str() {
                "high" => format!("[{}]", belief.belief_type),
                other => format!("[{}, {}]", belief.belief_type, other),
            };
            out.push_str(&format!("- {} {}\n", belief.content, confidence_tag));
        }
    }

    out
}
