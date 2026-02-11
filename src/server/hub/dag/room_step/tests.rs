#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::super::extract_room_outputs_from_speakers;
    use crate::server::executors::room::SpeakerResult;
    use crate::server::hub::dag::resolve_dot_path;

    #[test]
    fn room_composite_envelope_structure() {
        let agent_a = Uuid::new_v4();
        let agent_b = Uuid::new_v4();

        let speakers = vec![
            SpeakerResult {
                agent_id: agent_a,
                agent_name: "Architect".into(),
                content: r#"{"recommendation": "use microservices"}"#.into(),
                input_tokens: 100,
                output_tokens: 50,
                speaker_order: 0,
            },
            SpeakerResult {
                agent_id: agent_b,
                agent_name: "Reviewer".into(),
                content: "I agree with the approach.".into(),
                input_tokens: 80,
                output_tokens: 30,
                speaker_order: 1,
            },
        ];

        let (envelope_data, output) =
            extract_room_outputs_from_speakers(&speakers, Some("room_out"));

        // Verify output variable name
        assert_eq!(output.variable_name, "room_out");

        // Verify composite structure has per-agent keys
        let key_a = format!("agent:{}", agent_a);
        let key_b = format!("agent:{}", agent_b);

        // Agent A returned valid JSON — should be parsed as object
        let val_a = resolve_dot_path(&envelope_data, &key_a).unwrap();
        assert_eq!(val_a["recommendation"], "use microservices");

        // Agent B returned plain text — should be stored as string
        let val_b = resolve_dot_path(&envelope_data, &key_b).unwrap();
        assert_eq!(val_b.as_str().unwrap(), "I agree with the approach.");

        // Nested access works through the port system
        let nested_path = format!("{}.recommendation", key_a);
        let nested = resolve_dot_path(&envelope_data, &nested_path).unwrap();
        assert_eq!(nested, "use microservices");
    }
}
