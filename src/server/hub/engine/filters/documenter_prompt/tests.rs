#[cfg(test)]
mod tests {
    use crate::db::ProtocolDocumentDefRow;
    use crate::server::hub::protocols::compilers::documenter::prompt::format_document_defs_section;
    use chrono::Utc;
    use uuid::Uuid;

    /// Helper to create a mock document def.
    fn make_def(name: &str, target_length: i32, description: &str) -> ProtocolDocumentDefRow {
        ProtocolDocumentDefRow {
            id: Uuid::new_v4(),
            step_id: Some(Uuid::new_v4()),
            name: name.to_string(),
            description: description.to_string(),
            target_length,
            display_order: 0,
            created_at: Utc::now(),
            protocol_id: None,
            document_id: None,
        }
    }

    #[test]
    fn format_section_includes_all_documents() {
        let defs = vec![
            make_def("API Docs", 4000, "REST API reference for the auth service"),
            make_def("Best Practices", 2000, "Modern Rust patterns"),
        ];

        let section = format_document_defs_section(&defs);

        assert!(section.contains("API Docs"));
        assert!(section.contains("4000 characters"));
        assert!(section.contains("Best Practices"));
        assert!(section.contains("2000 characters"));
        assert!(section.contains("2 document(s)"));
        assert!(section.contains("## Document Definitions"));
    }

    #[test]
    fn format_section_empty_description_shows_placeholder() {
        let defs = vec![make_def("Test Doc", 1000, "")];

        let section = format_document_defs_section(&defs);

        assert!(section.contains("(no description provided)"));
        assert!(section.contains("Test Doc"));
        assert!(section.contains("1000 characters"));
    }
}
