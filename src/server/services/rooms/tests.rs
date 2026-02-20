#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::traits::{CreateRoomInput, MockRoomRepo};
    use crate::db::{RoomRow, RoomTranscriptEntry};
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
        let mut s = step();
        s.execution_mode = "room".into();
        s.output_variable_name = Some("debate_output".into());
        s.name = Some("debate".into());

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

        let output = build_room_step_output(&transcript, &s);

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
        let mut s = step();
        s.execution_mode = "room".into();
        s.name = Some("room_step".into());

        let transcript = vec![];

        let output = build_room_step_output(&transcript, &s);
        assert_eq!(output.variable_name, "");
        assert!(output
            .structured_output
            .unwrap()
            .as_object()
            .unwrap()
            .is_empty());
    }
}
