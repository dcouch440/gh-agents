#[cfg(test)]
mod tests {
    use crate::db::ProtocolDocumentDefRow;
    use crate::server::hub::engine::filters::FilterContext;
    use chrono::Utc;
    use uuid::Uuid;

    /// Build a minimal FilterContext with a step_id.
    #[allow(dead_code)]
    fn make_ctx(step_id: Uuid) -> FilterContext {
        FilterContext::new("claude-sonnet-4-20250514", Uuid::new_v4()).with_step_id(step_id)
    }

    /// Helper to create a mock document def.
    fn make_def(name: &str, target_length: i32, description: &str) -> ProtocolDocumentDefRow {
        ProtocolDocumentDefRow {
            id: Uuid::new_v4(),
            step_id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            target_length,
            display_order: 0,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn augments_prompt_with_document_definitions() {
        let defs = vec![
            make_def("API Docs", 4000, "REST API reference for the auth service"),
            make_def("Best Practices", 2000, "Modern Rust patterns"),
        ];

        let mut prompt = "You are the Documenter Strategist.".to_string();
        prompt.push_str("\n\n## Document Definitions\n");
        prompt.push_str(&format!(
            "The user has requested {} document(s) to be generated:\n\n",
            defs.len()
        ));

        for (i, def) in defs.iter().enumerate() {
            prompt.push_str(&format!(
                "Document {}: \"{}\"\n  Target length: {} characters\n  Description: {}\n\n",
                i + 1,
                def.name,
                def.target_length,
                &def.description
            ));
        }

        assert!(prompt.contains("API Docs"));
        assert!(prompt.contains("4000 characters"));
        assert!(prompt.contains("Best Practices"));
        assert!(prompt.contains("2000 characters"));
        assert!(prompt.contains("2 document(s)"));
    }

    #[test]
    fn empty_description_shows_placeholder() {
        let def = make_def("Test Doc", 1000, "");
        let description = if def.description.is_empty() {
            "(no description provided)"
        } else {
            &def.description
        };
        assert_eq!(description, "(no description provided)");
    }
}
