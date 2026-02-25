//! Context document utilities for protocol pipelines.
//!
//! Provides a shared `ContextDocument` type and XML-formatted context block
//! builder. Used by protocol executors to selectively inject upstream documents
//! into LLM prompts based on short ID assignments.

/// A context document available to protocol pipeline phases.
///
/// Loaded from agent context, step documents, or upstream steps,
/// then selectively injected into LLM prompts based on short_id assignments.
#[derive(Debug, Clone)]
pub struct ContextDocument {
    /// Short identifier (first 8 chars of UUID).
    pub short_id: String,
    /// Document title.
    pub title: String,
    /// Document content.
    pub content: String,
}

/// Build a `<context>` XML block from assigned document IDs.
///
/// - If `all_docs` is empty, returns empty string (no context exists).
/// - If `assigned_ids` is empty, includes ALL docs (backward compat).
/// - If `assigned_ids` is non-empty, filters to only matching docs.
pub fn build_context_block(assigned_ids: &[String], all_docs: &[ContextDocument]) -> String {
    if all_docs.is_empty() {
        return String::new();
    }

    let relevant_docs: Vec<&ContextDocument> = if assigned_ids.is_empty() {
        all_docs.iter().collect()
    } else {
        all_docs
            .iter()
            .filter(|d| assigned_ids.contains(&d.short_id))
            .collect()
    };

    if relevant_docs.is_empty() {
        return String::new();
    }

    let mut block = String::from("<context>");
    for doc in relevant_docs {
        block.push_str(&format!(
            "\n<document_{} title=\"{}\">\n{}\n</document_{}>",
            doc.short_id, doc.title, doc.content, doc.short_id
        ));
    }
    block.push_str("\n</context>");
    block
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
