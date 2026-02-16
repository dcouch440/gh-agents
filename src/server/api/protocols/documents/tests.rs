#[cfg(test)]
mod tests {
    use crate::server::api::protocols::documents::*;

    #[test]
    fn create_request_deserializes_required_fields() {
        let json = r#"{"name": "API Reference"}"#;
        let req: CreateProtocolDocDefRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "API Reference");
        assert_eq!(req.description, "");
        assert_eq!(req.target_length, 2000);
        assert_eq!(req.display_order, 0);
    }

    #[test]
    fn create_request_deserializes_all_fields() {
        let json = r#"{
            "name": "Architecture Guide",
            "description": "High-level system architecture",
            "target_length": 5000,
            "display_order": 2
        }"#;
        let req: CreateProtocolDocDefRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Architecture Guide");
        assert_eq!(req.description, "High-level system architecture");
        assert_eq!(req.target_length, 5000);
        assert_eq!(req.display_order, 2);
    }

    #[test]
    fn update_request_partial() {
        let json = r#"{"name": "Updated Name"}"#;
        let req: UpdateProtocolDocDefRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("Updated Name"));
        assert!(req.description.is_none());
        assert!(req.target_length.is_none());
    }

    #[test]
    fn update_request_all_fields() {
        let json = r#"{
            "name": "New Name",
            "description": "New description",
            "target_length": 3000
        }"#;
        let req: UpdateProtocolDocDefRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("New Name"));
        assert_eq!(req.description.as_deref(), Some("New description"));
        assert_eq!(req.target_length, Some(3000));
    }

    #[test]
    fn response_from_row() {
        let row = crate::db::ProtocolDocumentDefRow {
            id: uuid::Uuid::new_v4(),
            step_id: None,
            name: "API Reference".to_string(),
            description: "Full API docs".to_string(),
            target_length: 5000,
            display_order: 1,
            created_at: chrono::Utc::now(),
            protocol_id: Some(uuid::Uuid::new_v4()),
            document_id: None,
            agent_roster_entry_id: None,
        };
        let protocol_id = row.protocol_id.unwrap();
        let resp = ProtocolDocDefResponse::from_row(row);
        assert_eq!(resp.name, "API Reference");
        assert_eq!(resp.protocol_id, protocol_id.to_string());
        assert_eq!(resp.target_length, 5000);
        assert_eq!(resp.display_order, 1);
    }
}
