//! Prompt builders and system prompt templates for the documenter pipeline.
//!
//! Contains prompt composition functions used by the research and write phases,
//! plus the static output builder.

use serde_json::Value as JsonValue;

pub(crate) const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Build the system prompt for a research phase LLM call.
pub(crate) fn build_research_system_prompt(doc_name: &str) -> String {
    format!(
        "You are a research assistant gathering information for a document titled \"{}\".\n\
         Use the available tools to gather comprehensive, accurate information.\n\
         Summarize your findings clearly — your output will be used by a writer to produce the final document.",
        doc_name
    )
}

/// Build the system prompt for a write phase LLM call.
pub(crate) fn build_writer_system_prompt(doc_name: &str) -> String {
    format!(
        "You are a technical writer. Produce a well-structured, comprehensive document \
         titled \"{}\". Write in clear, professional prose. Use markdown formatting.",
        doc_name
    )
}

/// Build a structured output JSON summarising document results.
///
/// Used by tests and the executor to create the final `StepOutput`.
pub fn build_documents_output(statuses: Vec<JsonValue>) -> JsonValue {
    serde_json::json!({ "documents": statuses })
}

/// Compose the user prompt for a write phase LLM call.
///
/// Combines the writer instructions (from strategy LLM), optional context
/// documents, and research findings into a single prompt.
pub(crate) fn compose_write_prompt(
    writer_prompt: &str,
    context_block: &str,
    research_content: &str,
) -> String {
    if context_block.is_empty() {
        format!(
            "{}\n\n---\n\nResearch findings:\n{}",
            writer_prompt, research_content
        )
    } else {
        format!(
            "{}\n\n{}\n\n---\n\nResearch findings:\n{}",
            writer_prompt, context_block, research_content
        )
    }
}

/// Compose the user prompt for a research phase LLM call.
///
/// Combines the research strategy (from strategy LLM) with optional context
/// documents.
pub(crate) fn compose_research_prompt(research_strategy: &str, context_block: &str) -> String {
    if context_block.is_empty() {
        research_strategy.to_string()
    } else {
        format!("{}\n\n{}", research_strategy, context_block)
    }
}
