#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::{CreateRoomInput, MockRoomRepo};
    use crate::db::{RoomRow, RoomTranscriptEntry, WorkflowStepRow};
    use crate::server::services::rooms::*;
    use crate::server::services::ServiceError;

    fn make_room(user_id: Uuid, name: &str) -> RoomRow {
        RoomRow {
            id: Uuid::new_v4(),
            user_id,
            collection_id: None,
            name: name.to_string(),
            gatekeeper_enabled: false,
            gatekeeper_model_id: "claude-sonnet-4-20250514".to_string(),
            max_speakers_per_turn: 1,
            max_turns: 5,
            tools_enabled: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_step(name: &str, output_var: Option<&str>) -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            agent_id: None,
            execution_mode: "room".to_string(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: output_var.map(|s| s.to_string()),
            interactive_agent_id: None,
            for_each_label_field: None,
            room_id: None,
            routing_mode: None,
            routing_field: None,
            display_order: 0,
            version: 1,
            reasoning_trace: false,
            verification_agent_ids: None,
            position_x: None,
            position_y: None,
            width: None,
            height: None,
            name: Some(name.to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            sub_workflow_template_id: None,
            child_workflow_id: None,
            is_designer_step: false,
        }
    }

    #[tokio::test]
    async fn create_room_rejects_empty_name() {
        let repo = MockRoomRepo::new();
        let user_id = Uuid::new_v4();

        let result = create_room(
            &repo,
            user_id,
            CreateRoomInput {
                user_id,
                collection_id: None,
                name: "  ".to_string(),
                gatekeeper_enabled: false,
                gatekeeper_model_id: "model".to_string(),
                max_speakers_per_turn: 1,
                max_turns: 5,
                tools_enabled: false,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_room_succeeds_with_valid_name() {
        let user_id = Uuid::new_v4();

        let mut repo = MockRoomRepo::new();
        repo.expect_create_room().returning(move |input| {
            Ok(RoomRow {
                id: Uuid::new_v4(),
                user_id: input.user_id,
                collection_id: input.collection_id,
                name: input.name,
                gatekeeper_enabled: input.gatekeeper_enabled,
                gatekeeper_model_id: input.gatekeeper_model_id,
                max_speakers_per_turn: input.max_speakers_per_turn,
                max_turns: input.max_turns,
                tools_enabled: input.tools_enabled,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        });

        let result = create_room(
            &repo,
            user_id,
            CreateRoomInput {
                user_id,
                collection_id: None,
                name: "Design Review".to_string(),
                gatekeeper_enabled: false,
                gatekeeper_model_id: "model".to_string(),
                max_speakers_per_turn: 1,
                max_turns: 5,
                tools_enabled: false,
            },
        )
        .await;
        let row = result.unwrap();
        assert_eq!(row.name, "Design Review");
    }

    #[tokio::test]
    async fn get_room_rejects_non_owner() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let room = make_room(owner_id, "Private Room");
        let room_id = room.id;
        let room_clone = room.clone();

        let mut repo = MockRoomRepo::new();
        repo.expect_get_room()
            .returning(move |_| Ok(Some(room_clone.clone())));

        let result = get_room(&repo, attacker_id, room_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn build_room_step_output_groups_by_agent_last_message() {
        let step = make_step("debate", Some("debate_output"));

        let transcript = vec![
            RoomTranscriptEntry {
                agent_name: "Alice".to_string(),
                role_description: "Analyst".to_string(),
                content: "First message from Alice".to_string(),
                speaker_order: Some(0),
                created_at: Utc::now(),
            },
            RoomTranscriptEntry {
                agent_name: "Bob".to_string(),
                role_description: "Reviewer".to_string(),
                content: r#"{"verdict": "approved"}"#.to_string(),
                speaker_order: Some(1),
                created_at: Utc::now(),
            },
            RoomTranscriptEntry {
                agent_name: "Alice".to_string(),
                role_description: "Analyst".to_string(),
                content: "Updated analysis from Alice".to_string(),
                speaker_order: Some(2),
                created_at: Utc::now(),
            },
        ];

        let output = build_room_step_output(&transcript, &step);

        assert_eq!(output.variable_name, "debate_output");

        let structured = output.structured_output.unwrap();
        let obj = structured.as_object().unwrap();

        // Alice's last message should be the plain string (not JSON-parseable)
        assert_eq!(
            obj.get("alice").unwrap(),
            &serde_json::Value::String("Updated analysis from Alice".to_string())
        );

        // Bob's message is valid JSON, so it should be parsed into a structured value
        let bob_val = obj.get("bob").unwrap();
        assert_eq!(bob_val["verdict"], "approved");
    }

    #[tokio::test]
    async fn build_room_step_output_defaults_variable_name() {
        let step = make_step("room_step", None);
        let transcript = vec![];

        let output = build_room_step_output(&transcript, &step);
        assert_eq!(output.variable_name, "");
        assert!(output
            .structured_output
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty());
    }
}
