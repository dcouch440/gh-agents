#[cfg(test)]
mod tests {
    use crate::server::api::document_defs::{CreateDocumentDefRequest, UpdateDocumentDefRequest};

    #[test]
    fn create_request_deserializes() {
        let json = r#"{"name":"API Docs","description":"REST API reference","target_length":4000}"#;
        let req: CreateDocumentDefRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "API Docs");
        assert_eq!(req.description, "REST API reference");
        assert_eq!(req.target_length, 4000);
    }

    #[test]
    fn create_request_defaults() {
        let json = r#"{"name":"Quick Doc"}"#;
        let req: CreateDocumentDefRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Quick Doc");
        assert_eq!(req.description, "");
        assert_eq!(req.target_length, 2000);
        assert_eq!(req.display_order, 0);
    }

    #[test]
    fn update_request_partial() {
        let json = r#"{"name":"Updated Name"}"#;
        let req: UpdateDocumentDefRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, Some("Updated Name".to_string()));
        assert!(req.description.is_none());
        assert!(req.target_length.is_none());
    }
}
