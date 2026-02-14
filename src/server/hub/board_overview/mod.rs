//! Board overview summary — Haiku distiller for cross-board awareness.
//!
//! After any assistant updates its notes, Haiku summarizes ALL assistant
//! notes across the workflow into a single paragraph. This summary is
//! injected into every assistant's system prompt so each node has ambient
//! awareness of the full board.

use tracing::info;
use uuid::Uuid;

use crate::llm::{
    AnthropicClient, AnthropicConfig, LLMProvider, LLMRequest, Message as LlmMessage,
};
use crate::server::state::AppState;

/// Max tokens for the board overview response.
/// One paragraph = ~100-150 tokens. Allow headroom.
const MAX_TOKENS_BOARD_OVERVIEW: u32 = 512;

const BOARD_OVERVIEW_SYSTEM_PROMPT: &str = r#"You summarize what a workflow board is doing.

You receive notes from multiple workflow steps. Each step is a different part of the pipeline (task forces, documenters, rooms, etc.). The notes were written by each step's configuration assistant during conversations with the user.

Produce ONE paragraph (3-5 sentences max) that describes:
- What the overall workflow does (the big picture)
- What each step contributes to the pipeline
- Any key constraints or technical details that affect multiple steps

Write for an AI assistant that needs ambient awareness of the full board. Be specific — name the actual technologies, domains, and deliverables mentioned in the notes. Do not be vague ("the team is working on a project") — be concrete ("the pipeline scans Python repos for auth vulnerabilities, generates a remediation guide, then a review panel debates priority").

If only one step has notes, still summarize it — the other steps haven't been configured yet.

Return ONLY the paragraph. No headers, no bullet points, no preamble."#;

/// Spawn a background board overview summarization.
/// Non-blocking — fires and forgets. Errors are logged, not propagated.
///
/// Called after any `update_notes` tool call completes.
pub fn spawn_board_overview_update(state: AppState, workflow_id: Uuid) {
    tokio::spawn(async move {
        if let Err(e) = regenerate_board_overview(&state, workflow_id).await {
            tracing::error!("Board overview update failed for workflow {workflow_id}: {e}");
        }
    });
}

/// Load all assistant notes across the workflow, summarize via Haiku,
/// store the result on the workflow row.
async fn regenerate_board_overview(
    state: &AppState,
    workflow_id: Uuid,
) -> Result<(), anyhow::Error> {
    // 1. Load all assistant notes in one query
    let all_notes = state
        .repos()
        .workflows
        .get_all_assistant_notes_for_workflow(workflow_id)
        .await?;

    // 2. Format as (label, content) pairs
    let notes_by_step: Vec<(String, String)> = all_notes
        .into_iter()
        .map(|(_step_id, name, mode, content)| {
            let step_name = name.as_deref().unwrap_or("(unnamed)");
            let label = format!("{step_name} ({mode})");
            (label, content)
        })
        .collect();

    // 3. If no notes exist anywhere, clear the summary
    if notes_by_step.is_empty() {
        state
            .repos()
            .workflows
            .update_board_overview_summary(workflow_id, "")
            .await?;
        return Ok(());
    }

    // 4. Format all notes as input for Haiku
    let formatted_input = format_notes_for_summarization(&notes_by_step);

    // 5. Call Haiku
    let config = AnthropicConfig::from_env()?;
    let client = AnthropicClient::new(config)?;

    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(formatted_input)],
    )
    .with_system(BOARD_OVERVIEW_SYSTEM_PROMPT)
    .with_max_tokens(MAX_TOKENS_BOARD_OVERVIEW);

    let response = client.send_message(request).await?;
    let summary = response.content.trim().to_string();

    info!(
        workflow_id = %workflow_id,
        steps_with_notes = notes_by_step.len(),
        summary_len = summary.len(),
        "Board overview summary updated"
    );

    // 6. Store on workflow
    state
        .repos()
        .workflows
        .update_board_overview_summary(workflow_id, &summary)
        .await?;

    Ok(())
}

/// Format all assistant notes across the board into Haiku input.
///
/// Each step's notes are wrapped with `[Step Name (mode)]` header.
pub(crate) fn format_notes_for_summarization(notes_by_step: &[(String, String)]) -> String {
    let mut out = String::new();
    for (i, (step_label, notes)) in notes_by_step.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        out.push_str(&format!("[{step_label}]\n{notes}"));
    }
    out
}

mod tests;
