//! Utility LLM helper functions for summarization and context extraction.
//!
//! These utilities call the active provider's utility-tier model for lightweight
//! text processing tasks: file summarization, document summaries, conversation
//! titles, and context extraction.

use crate::llm::{LLMRequest, Message as LlmMessage};

mod tests;

/// Call the utility model to summarize a file for the orchestrator context.
pub async fn haiku_read_file(prompt: &str) -> Option<String> {
    let client = crate::llm::create_utility_client().ok()?;

    let request = LLMRequest::new(crate::constants::MODEL_TIER3, vec![LlmMessage::user(prompt.to_string())])
        .with_system(
            "You are a code reader. Given a source file, extract and return the most relevant content. \
         Include function signatures, struct/type definitions, key logic, and imports. \
         Use the original code when possible — quote exact lines for precision. \
         If a focus area is specified, prioritize content related to it. \
         Be concise but preserve technical accuracy. Do not add commentary.",
        )
        .with_max_tokens(crate::constants::MAX_TOKENS_FILE_READ);

    match client.send_message(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            tracing::warn!("Utility file read failed: {}", e);
            None
        }
    }
}

/// Call the utility model to generate a short summary for search indexing.
pub async fn haiku_summarize(content: &str) -> Option<String> {
    let client = crate::llm::create_utility_client().ok()?;

    let truncated: String = content
        .chars()
        .take(crate::constants::TRUNCATE_SUMMARY_INPUT)
        .collect();
    let request = LLMRequest::new(
        crate::constants::MODEL_TIER3,
        vec![LlmMessage::user(truncated)],
    )
    .with_system("Summarize this document in 2-3 sentences. Include key entities, topics, and actions. This summary is used for search indexing.")
    .with_max_tokens(crate::constants::MAX_TOKENS_SUMMARIZE);

    match client.send_message(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            tracing::warn!("Utility summarization failed: {}", e);
            None
        }
    }
}

/// Call the utility model to generate a short title for a conversation.
pub async fn haiku_summarize_title(content: &str) -> Option<String> {
    let client = crate::llm::create_utility_client().ok()?;

    let truncated: String = content
        .chars()
        .take(crate::constants::TRUNCATE_TITLE_INPUT)
        .collect();
    let request = LLMRequest::new(crate::constants::MODEL_TIER3, vec![LlmMessage::user(truncated)])
        .with_system("Generate a short title (3-6 words) for this conversation. This title appears in sidebar navigation. Return the title as plain text, without quotes or trailing punctuation.")
        .with_max_tokens(crate::constants::MAX_TOKENS_TITLE);

    match client.send_message(request).await {
        Ok(resp) => {
            let title = resp.content.trim().to_string();
            if title.is_empty() {
                None
            } else {
                Some(title)
            }
        }
        Err(e) => {
            tracing::warn!("Utility title generation failed: {}", e);
            None
        }
    }
}

/// Call the utility model to extract relevant context from a conversation summary
/// based on the user's current message.
pub async fn haiku_extract_context(summary: &str, current_message: &str) -> Option<String> {
    let client = crate::llm::create_utility_client().ok()?;

    let user_text = format!(
        "Summary:\n{}\n\nCurrent message:\n{}",
        summary, current_message
    );
    let request = LLMRequest::new(
        crate::constants::MODEL_TIER3,
        vec![LlmMessage::user(user_text)],
    )
    .with_system("Extract relevant context from a conversation summary based on the user's current message. The extracted context will be prepended to a new conversation turn. Return 2-4 sentences that are directly relevant to the current request. If nothing is relevant, return 'No prior context needed.'")
    .with_max_tokens(crate::constants::MAX_TOKENS_CONTEXT);

    match client.send_message(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            tracing::warn!("Utility context extraction failed: {}", e);
            None
        }
    }
}
