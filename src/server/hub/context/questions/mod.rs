//! Question extraction — Tier 3 compresses node responses into status + question.
//!
//! After each step chat completion, Tier 3 extracts a compressed status and
//! optional pending question from the node assistant's latest response.
//! These are stored in `step_question_state` and rendered in `<board_state>`
//! as `<status>` and `<asking>` tags for the manager (L1).

use std::collections::HashMap;

use serde::Deserialize;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::protocols::{roles, vars};
use crate::db::ChatMessageRow;
use crate::llm::{LLMRequest, Message as LlmMessage};
use crate::server::hub::protocols::json_utils::parse_structured_output;
use crate::server::state::AppState;

mod tests;

/// Max tokens for the extraction response (one JSON line).
const MAX_TOKENS_EXTRACTION: u32 = 256;

/// Number of recent messages to load for extraction context.
const MAX_MESSAGES_FOR_EXTRACTION: u32 = 6;

// ── Spawn helper ────────────────────────────────────────────────────────

/// Spawn a background question extraction for a step's chat conversation.
/// Non-blocking — fires and forgets. Errors are logged, not propagated.
pub fn spawn_question_extraction(
    state: AppState,
    _workflow_id: Uuid,
    step_id: Uuid,
    session_id: Uuid,
) {
    tokio::spawn(async move {
        if let Err(e) = extract_and_store(&state, step_id, session_id).await {
            tracing::error!("Question extraction failed for step {step_id}: {e}");
        }
    });
}

// ── Extraction ──────────────────────────────────────────────────────────

/// Load recent conversation, extract status + question via Tier 3, store in DB.
async fn extract_and_store(
    state: &AppState,
    step_id: Uuid,
    session_id: Uuid,
) -> Result<(), anyhow::Error> {
    // 1. Load step name
    let step = state
        .repos()
        .workflows
        .get_step(step_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Step {step_id} not found"))?;

    let node_name = step.name.as_deref().unwrap_or("(unnamed)");

    // 2. Load last N messages
    let messages = state
        .repos()
        .sessions
        .get_session_history(session_id, MAX_MESSAGES_FOR_EXTRACTION)
        .await?;

    // 3. Skip if no assistant response yet
    if !messages.iter().any(|m| m.role == "assistant") {
        return Ok(());
    }

    // 4. Format conversation
    let conversation = format_conversation(&messages);

    // 5. Resolve prompt template
    let mut vars_map = HashMap::new();
    vars_map.insert(
        vars::question_extraction::NODE_NAME.to_string(),
        node_name.to_string(),
    );
    vars_map.insert(
        vars::question_extraction::CONVERSATION.to_string(),
        conversation,
    );
    let resolved = roles::QUESTION_EXTRACTOR.resolve(&vars_map);

    // 6. Call utility model (Tier 3)
    let client = crate::llm::create_utility_client()?;
    let request = LLMRequest::new(
        crate::constants::MODEL_TIER3,
        vec![LlmMessage::user(resolved.user_prompt)],
    )
    .with_system(&resolved.system_prompt)
    .with_max_tokens(MAX_TOKENS_EXTRACTION);

    let response = client.send_message(request).await?;

    // 7. Parse JSON
    let (status, question) = match parse_extraction_output(&response.content) {
        Ok(result) => result,
        Err(e) => {
            warn!(
                step_id = %step_id,
                raw = %response.content,
                "Question extraction parse failed: {e}"
            );
            return Ok(());
        }
    };

    // 8. Log + Store
    let has_question = question.is_some();

    state
        .repos()
        .workflows
        .upsert_step_question_state(step_id, &status, question)
        .await?;

    info!(
        step_id = %step_id,
        status = %status,
        has_question = has_question,
        "Question extraction complete"
    );

    Ok(())
}

// ── Formatting ──────────────────────────────────────────────────────────

fn format_conversation(messages: &[ChatMessageRow]) -> String {
    messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

// ── Parsing ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ExtractionOutput {
    status: String,
    question: Option<String>,
}

pub(crate) fn parse_extraction_output(
    content: &str,
) -> Result<(String, Option<String>), anyhow::Error> {
    // Try direct JSON parse first
    if let Ok(out) = serde_json::from_str::<ExtractionOutput>(content.trim()) {
        return Ok((out.status, out.question));
    }
    // Fallback: extract JSON from markdown code fences
    if let Some(json) = parse_structured_output(content) {
        let out: ExtractionOutput = serde_json::from_value(json)?;
        return Ok((out.status, out.question));
    }
    anyhow::bail!("Failed to parse question extraction output")
}
