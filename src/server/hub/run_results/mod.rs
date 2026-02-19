//! Run results summarizer — LLM distiller for post-execution step summaries.
//!
//! After a step executes, an LLM summarizes what it produced. The summary is
//! stored on the step row and later injected into assistant system prompts via
//! `<run_context>` so the assistant has awareness of what upstream/downstream
//! steps have produced.
//!
//! Provider-agnostic: uses `state.provider()` (`Arc<dyn LLMProvider>`) so it
//! works with whatever provider is configured. Fire-and-forget with
//! cancel-and-replace via `CancellationToken`.

use dashmap::DashMap;
use tokio_util::sync::CancellationToken;
use tracing::info;
use uuid::Uuid;

use crate::llm::{LLMRequest, Message as LlmMessage};
use crate::server::state::AppState;

/// Max tokens for the run results summary response.
const MAX_TOKENS_RUN_SUMMARY: u32 = 512;

/// Max characters of output to send to the LLM (truncate before calling).
const MAX_OUTPUT_CHARS: usize = 50_000;

/// Model to use for summarization. Uses the default utility model.
const SUMMARIZER_MODEL: &str = crate::constants::MODEL_HAIKU;

const RUN_RESULTS_SYSTEM_PROMPT: &str = r#"You summarize the output of a workflow step execution.

You receive the raw output from a step that just completed. Describe what the step produced in 2-4 sentences:
- What kind of data or content was generated
- Key data points, entities, or conclusions
- The shape/structure of the output (list, object, text, etc.)

Be concrete and specific. Reference actual values, names, and numbers from the output.
Do NOT describe what the step was supposed to do — describe what it actually produced.

Return ONLY the summary. No headers, no bullet points, no preamble."#;

/// In-memory token map for cancel-and-replace semantics.
/// Each step_id maps to a CancellationToken for the in-flight summarization.
pub(crate) type RunResultsTokens = DashMap<Uuid, CancellationToken>;

/// Create a new token map (called once during AppState construction).
pub(crate) fn new_run_results_tokens() -> RunResultsTokens {
    DashMap::new()
}

/// Spawn a background run results summarization for a step.
///
/// Non-blocking — fires and forgets. Cancel-and-replace: if a previous
/// summarization is in-flight for this step, it's cancelled first.
pub fn spawn_run_results_summary(
    state: AppState,
    tokens: &RunResultsTokens,
    step_id: Uuid,
    output_content: String,
) {
    // Cancel any in-flight summarization for this step
    if let Some((_, old_token)) = tokens.remove(&step_id) {
        old_token.cancel();
    }

    let token = CancellationToken::new();
    tokens.insert(step_id, token.clone());

    let tokens_ref = tokens.clone();
    tokio::spawn(async move {
        let result = summarize_step_output(&state, step_id, &output_content, &token).await;

        // Clean up token from map
        tokens_ref.remove(&step_id);

        if let Err(e) = result {
            tracing::warn!(step_id = %step_id, "Run results summarization failed: {e}");
        }
    });
}

/// Summarize step output via LLM, store result on step row.
async fn summarize_step_output(
    state: &AppState,
    step_id: Uuid,
    output_content: &str,
    cancel: &CancellationToken,
) -> Result<(), anyhow::Error> {
    // Skip empty output
    if output_content.trim().is_empty() {
        return Ok(());
    }

    // Truncate long output
    let truncated = if output_content.len() > MAX_OUTPUT_CHARS {
        &output_content[..MAX_OUTPUT_CHARS]
    } else {
        output_content
    };

    // Get provider
    let provider = state
        .provider()
        .ok_or_else(|| anyhow::anyhow!("No LLM provider configured"))?;

    let request = LLMRequest::new(
        SUMMARIZER_MODEL,
        vec![LlmMessage::user(truncated.to_string())],
    )
    .with_system(RUN_RESULTS_SYSTEM_PROMPT)
    .with_max_tokens(MAX_TOKENS_RUN_SUMMARY);

    let response = provider.send_message(request).await?;
    let summary = response.content.trim().to_string();

    // Check cancellation before writing to DB
    if cancel.is_cancelled() {
        return Ok(());
    }

    info!(
        step_id = %step_id,
        summary_len = summary.len(),
        "Run results summary updated"
    );

    state
        .repos()
        .workflows
        .update_run_results_summary(step_id, &summary)
        .await?;

    Ok(())
}

mod tests;
