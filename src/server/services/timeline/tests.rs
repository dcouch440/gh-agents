#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::TimelineRow;
    use crate::server::services::timeline::{map_row_to_entries, TimelineEntryKind};

    fn make_timeline_row(role: &str, content: &str) -> TimelineRow {
        TimelineRow {
            id: Uuid::new_v4(),
            ts: Utc::now(),
            role: role.to_string(),
            content: content.to_string(),
            reasoning: None,
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
    fn a_system_message_is_one_entry() {
        let entries = map_row_to_entries(make_timeline_row("system", "You are an AI assistant."));
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].kind, TimelineEntryKind::SystemPrompt));
        assert!(entries[0].tool_name.is_none());
    }

    #[test]
    fn a_user_message_is_one_entry() {
        let entries = map_row_to_entries(make_timeline_row("user", "Analyze this data."));
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].kind, TimelineEntryKind::UserMessage));
    }

    #[test]
    fn assistant_prose_is_one_entry() {
        let entries = map_row_to_entries(make_timeline_row("assistant", "Based on my analysis..."));
        assert_eq!(entries.len(), 1);
        assert!(matches!(
            entries[0].kind,
            TimelineEntryKind::AssistantMessage
        ));
    }

    #[test]
    fn a_tool_result_keeps_its_call_id() {
        let mut row = make_timeline_row("tool", "Found 5 results...");
        row.tool_call_id = Some("call_123".to_string());
        let entries = map_row_to_entries(row);
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].kind, TimelineEntryKind::ToolResult));
        assert_eq!(entries[0].tool_call_id.as_deref(), Some("call_123"));
    }

    #[test]
    fn a_lone_tool_call_yields_its_name_and_bare_payload() {
        let entries = map_row_to_entries(make_timeline_row(
            "assistant",
            "tool_use: search_web {\"query\": \"rust async\"}",
        ));
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].kind, TimelineEntryKind::ToolCall));
        assert_eq!(entries[0].tool_name.as_deref(), Some("search_web"));
        // The payload alone — the marker and name are stripped so the frontend
        // parses it as JSON instead of rendering the raw line.
        assert_eq!(entries[0].content, "{\"query\": \"rust async\"}");
    }

    /// The bug: a row holding two calls used to collapse into a single entry,
    /// so the frontend's call counter fell one behind its result counter and
    /// every later card rendered a different call's result.
    #[test]
    fn a_row_with_two_calls_yields_two_entries() {
        let entries = map_row_to_entries(make_timeline_row(
            "assistant",
            "tool_use: read_file {\"path\": \"/src/main.rs\"}\ntool_use: search_web {\"q\": \"test\"}",
        ));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].tool_name.as_deref(), Some("read_file"));
        assert_eq!(entries[1].tool_name.as_deref(), Some("search_web"));
        assert!(entries
            .iter()
            .all(|e| matches!(e.kind, TimelineEntryKind::ToolCall)));
    }

    /// The exact shape that broke workflow b7f89491: prose, then two parallel
    /// searches. `starts_with` missed the marker entirely, so this row scored
    /// zero call entries against two results.
    #[test]
    fn prose_before_calls_still_yields_every_call() {
        let entries = map_row_to_entries(make_timeline_row(
            "assistant",
            "I'll research this systematically, starting with searches.\n\
             tool_use: brave_search {\"query\": \"nicotine cardiovascular\"}\n\
             tool_use: brave_search {\"query\": \"nicotine addiction\"}",
        ));
        assert_eq!(entries.len(), 3);
        assert!(matches!(
            entries[0].kind,
            TimelineEntryKind::AssistantMessage
        ));
        assert!(entries[0].content.starts_with("I'll research"));
        assert_eq!(entries[1].tool_name.as_deref(), Some("brave_search"));
        assert_eq!(entries[2].tool_name.as_deref(), Some("brave_search"));
    }

    #[test]
    fn split_entries_get_distinct_but_stable_ids() {
        let row = make_timeline_row("assistant", "thinking\ntool_use: a {}\ntool_use: b {}");
        let first = map_row_to_entries(row.clone());
        let again = map_row_to_entries(row);

        let ids: Vec<_> = first.iter().map(|e| e.id).collect();
        assert_eq!(ids.len(), 3);
        assert_ne!(ids[0], ids[1]);
        assert_ne!(ids[1], ids[2]);
        // Stable across refetches, or the frontend remounts every card on poll.
        assert_eq!(ids, again.iter().map(|e| e.id).collect::<Vec<_>>());
    }

    /// A single-call row can hand its id over; a two-call row cannot say which
    /// call the id belongs to, so it leaves them unset and lets the frontend
    /// pair positionally — which is correct now that the counts agree.
    #[test]
    fn only_an_unambiguous_row_passes_its_call_id_through() {
        let mut one = make_timeline_row("assistant", "tool_use: a {}");
        one.tool_call_id = Some("call_1".to_string());
        assert_eq!(
            map_row_to_entries(one)[0].tool_call_id.as_deref(),
            Some("call_1")
        );

        let mut two = make_timeline_row("assistant", "tool_use: a {}\ntool_use: b {}");
        two.tool_call_id = Some("call_1".to_string());
        assert!(map_row_to_entries(two)
            .iter()
            .all(|e| e.tool_call_id.is_none()));
    }

    #[test]
    fn a_call_with_no_payload_still_parses() {
        let entries = map_row_to_entries(make_timeline_row("assistant", "tool_use: think"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tool_name.as_deref(), Some("think"));
        assert_eq!(entries[0].content, "");
    }
}
