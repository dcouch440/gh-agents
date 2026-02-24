#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::TimelineRow;
    use crate::server::services::timeline::{classify_message, TimelineEntryKind};

    fn make_timeline_row(role: &str, content: &str) -> TimelineRow {
        TimelineRow {
            id: Uuid::new_v4(),
            ts: Utc::now(),
            role: role.to_string(),
            content: content.to_string(),
            tool_call_id: None,
            input_tokens: 0,
            output_tokens: 0,
            agent_execution_id: Uuid::new_v4(),
            execution_type: "dag_step".to_string(),
            step_id: Some(Uuid::new_v4()),
            step_name: Some("Worker".to_string()),
            agent_name: Some("Scanner".to_string()),
            agent_status: "completed".to_string(),
        }
    }

    #[test]
    fn classify_system_message() {
        let row = make_timeline_row("system", "You are an AI assistant.");
        let (kind, tool, _) = classify_message(&row);
        assert!(matches!(kind, TimelineEntryKind::SystemPrompt));
        assert!(tool.is_none());
    }

    #[test]
    fn classify_user_message() {
        let row = make_timeline_row("user", "Analyze this data.");
        let (kind, tool, _) = classify_message(&row);
        assert!(matches!(kind, TimelineEntryKind::UserMessage));
        assert!(tool.is_none());
    }

    #[test]
    fn classify_assistant_text() {
        let row = make_timeline_row("assistant", "Based on my analysis...");
        let (kind, tool, _) = classify_message(&row);
        assert!(matches!(kind, TimelineEntryKind::AssistantMessage));
        assert!(tool.is_none());
    }

    #[test]
    fn classify_assistant_tool_call() {
        let row = make_timeline_row(
            "assistant",
            "tool_use: search_web {\"query\": \"rust async\"}",
        );
        let (kind, tool, _) = classify_message(&row);
        assert!(matches!(kind, TimelineEntryKind::ToolCall));
        assert_eq!(tool.unwrap(), "search_web");
    }

    #[test]
    fn classify_tool_result() {
        let mut row = make_timeline_row("tool", "Found 5 results...");
        row.tool_call_id = Some("call_123".to_string());
        let (kind, _, _) = classify_message(&row);
        assert!(matches!(kind, TimelineEntryKind::ToolResult));
    }

    #[test]
    fn classify_multi_tool_call_message() {
        let row = make_timeline_row(
            "assistant",
            "tool_use: read_file {\"path\": \"/src/main.rs\"}\ntool_use: search_web {\"q\": \"test\"}",
        );
        let (kind, tool, _) = classify_message(&row);
        assert!(matches!(kind, TimelineEntryKind::ToolCall));
        // First tool name extracted
        assert_eq!(tool.unwrap(), "read_file");
    }
}
