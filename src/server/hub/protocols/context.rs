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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context_docs() -> Vec<ContextDocument> {
        vec![
            ContextDocument {
                short_id: "550e8400".into(),
                title: "API Spec".into(),
                content: "OpenAPI specification content".into(),
            },
            ContextDocument {
                short_id: "a1b2c3d4".into(),
                title: "Style Guide".into(),
                content: "Style guide content".into(),
            },
            ContextDocument {
                short_id: "deadbeef".into(),
                title: "Architecture".into(),
                content: "Architecture overview".into(),
            },
        ]
    }

    #[test]
    fn empty_docs_returns_empty() {
        let result = build_context_block(&[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn empty_docs_with_ids_returns_empty() {
        let result = build_context_block(&["550e8400".into()], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn empty_ids_includes_all_docs() {
        let docs = make_context_docs();
        let result = build_context_block(&[], &docs);
        assert!(result.contains("<context>"));
        assert!(result.contains("</context>"));
        assert!(result.contains("<document_550e8400"));
        assert!(result.contains("<document_a1b2c3d4"));
        assert!(result.contains("<document_deadbeef"));
        assert!(result.contains("API Spec"));
        assert!(result.contains("Style Guide"));
        assert!(result.contains("Architecture"));
    }

    #[test]
    fn filters_by_assigned_ids() {
        let docs = make_context_docs();
        let ids = vec!["550e8400".into(), "deadbeef".into()];
        let result = build_context_block(&ids, &docs);
        assert!(result.contains("<document_550e8400"));
        assert!(result.contains("<document_deadbeef"));
        assert!(!result.contains("<document_a1b2c3d4"));
        assert!(result.contains("API Spec"));
        assert!(!result.contains("Style Guide"));
        assert!(result.contains("Architecture"));
    }

    #[test]
    fn no_matching_ids_returns_empty() {
        let docs = make_context_docs();
        let ids = vec!["00000000".into()];
        let result = build_context_block(&ids, &docs);
        assert!(result.is_empty());
    }

    #[test]
    fn single_doc_format() {
        let docs = vec![ContextDocument {
            short_id: "abcd1234".into(),
            title: "Test Doc".into(),
            content: "Test content".into(),
        }];
        let result = build_context_block(&[], &docs);
        assert!(result.starts_with("<context>"));
        assert!(result.ends_with("</context>"));
        assert!(result.contains("<document_abcd1234 title=\"Test Doc\">"));
        assert!(result.contains("Test content"));
        assert!(result.contains("</document_abcd1234>"));
    }

    #[test]
    fn upstream_docs_included() {
        let upstream_docs = vec![
            ContextDocument {
                short_id: "up000001".into(),
                title: "Project Requirements".into(),
                content: "The system must support real-time notifications.".into(),
            },
            ContextDocument {
                short_id: "up000002".into(),
                title: "API Constraints".into(),
                content: "Rate limit: 100 req/s per user.".into(),
            },
        ];

        let result = build_context_block(&[], &upstream_docs);
        assert!(result.contains("<context>"));
        assert!(result.contains("<document_up000001"));
        assert!(result.contains("Project Requirements"));
        assert!(result.contains("real-time notifications"));
        assert!(result.contains("<document_up000002"));
        assert!(result.contains("API Constraints"));
        assert!(result.contains("Rate limit"));
    }

    #[test]
    fn filters_upstream_by_id() {
        let docs = vec![
            ContextDocument {
                short_id: "up000001".into(),
                title: "Included".into(),
                content: "This should appear.".into(),
            },
            ContextDocument {
                short_id: "up000002".into(),
                title: "Excluded".into(),
                content: "This should not appear.".into(),
            },
        ];

        let ids = vec!["up000001".into()];
        let result = build_context_block(&ids, &docs);
        assert!(result.contains("<document_up000001"));
        assert!(result.contains("Included"));
        assert!(!result.contains("<document_up000002"));
        assert!(!result.contains("Excluded"));
    }
}
