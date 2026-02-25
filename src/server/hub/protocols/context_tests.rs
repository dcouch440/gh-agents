#[cfg(test)]
mod tests {
    use crate::server::hub::protocols::context::*;

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
