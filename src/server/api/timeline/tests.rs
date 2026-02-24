#[cfg(test)]
mod tests {
    use crate::server::api::timeline::TimelineEntryResponse;
    use crate::server::services::timeline::{TimelineEntry, TimelineEntryKind};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn entry_response_from_service_entry() {
        let entry = TimelineEntry {
            id: Uuid::new_v4(),
            ts: Utc::now(),
            kind: TimelineEntryKind::ToolCall,
            step_name: Some("Worker".to_string()),
            agent_name: Some("Scanner".to_string()),
            agent_execution_id: Uuid::new_v4(),
            content: "tool_use: search_web {}".to_string(),
            tool_name: Some("search_web".to_string()),
            tool_call_id: None,
            input_tokens: 100,
            output_tokens: 50,
        };

        let response: TimelineEntryResponse = entry.into();
        assert_eq!(response.kind, "tool_call");
        assert_eq!(response.step_name.unwrap(), "Worker");
        assert_eq!(response.agent_name.unwrap(), "Scanner");
        assert_eq!(response.tool_name.unwrap(), "search_web");
    }

    #[test]
    fn entry_response_kind_serialization() {
        let kinds = vec![
            (TimelineEntryKind::SystemPrompt, "system_prompt"),
            (TimelineEntryKind::UserMessage, "user_message"),
            (TimelineEntryKind::AssistantMessage, "assistant_message"),
            (TimelineEntryKind::ToolCall, "tool_call"),
            (TimelineEntryKind::ToolResult, "tool_result"),
        ];

        for (kind, expected) in kinds {
            let entry = TimelineEntry {
                id: Uuid::new_v4(),
                ts: Utc::now(),
                kind,
                step_name: None,
                agent_name: None,
                agent_execution_id: Uuid::new_v4(),
                content: String::new(),
                tool_name: None,
                tool_call_id: None,
                input_tokens: 0,
                output_tokens: 0,
            };
            let response: TimelineEntryResponse = entry.into();
            assert_eq!(response.kind, expected);
        }
    }
}
