#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::MockToolCapabilityRepo;
    use crate::db::ToolRow;
    use crate::server::hub::capability_resolver::resolve_capabilities_to_tools;

    fn test_tool_row(name: &str) -> ToolRow {
        ToolRow {
            id: Uuid::new_v4(),
            name: name.to_string(),
            display_name: name.to_string(),
            description: format!("{} tool", name),
            parameters: serde_json::json!({}),
            created_at: Utc::now(),
            version: 1,
        }
    }

    #[tokio::test]
    async fn empty_capabilities_returns_empty() {
        let repo = MockToolCapabilityRepo::new();
        let (tools, names) = resolve_capabilities_to_tools(&[], &repo).await.unwrap();
        assert!(tools.is_empty());
        assert!(names.is_empty());
    }

    #[tokio::test]
    async fn resolves_single_capability_to_tools() {
        let mut repo = MockToolCapabilityRepo::new();
        repo.expect_get_tools_by_capabilities()
            .withf(|keys| keys == ["file_read"])
            .returning(|_| Ok(vec![test_tool_row("read_file")]));

        let (tools, names) = resolve_capabilities_to_tools(&["file_read".to_string()], &repo)
            .await
            .unwrap();

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "read_file");
        assert_eq!(names, vec!["read_file"]);
    }

    #[tokio::test]
    async fn resolves_multiple_capabilities() {
        let mut repo = MockToolCapabilityRepo::new();
        repo.expect_get_tools_by_capabilities().returning(|_| {
            Ok(vec![
                test_tool_row("read_file"),
                test_tool_row("web_research"),
            ])
        });

        let keys = vec!["file_read".to_string(), "web_search".to_string()];
        let (tools, names) = resolve_capabilities_to_tools(&keys, &repo).await.unwrap();

        assert_eq!(tools.len(), 2);
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"read_file"));
        assert!(tool_names.contains(&"web_research"));
        assert_eq!(names.len(), 2);
    }

    #[tokio::test]
    async fn deduplicates_tools() {
        let mut repo = MockToolCapabilityRepo::new();
        // DB returns the same tool twice (assigned to multiple capabilities)
        repo.expect_get_tools_by_capabilities()
            .returning(|_| Ok(vec![test_tool_row("read_file"), test_tool_row("read_file")]));

        let keys = vec!["file_read".to_string(), "code_analysis".to_string()];
        let (tools, _) = resolve_capabilities_to_tools(&keys, &repo).await.unwrap();

        // DISTINCT in SQL should prevent this, but the function also deduplicates
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "read_file");
    }

    #[tokio::test]
    async fn unregistered_tool_names_are_skipped() {
        let mut repo = MockToolCapabilityRepo::new();
        repo.expect_get_tools_by_capabilities().returning(|_| {
            Ok(vec![
                test_tool_row("read_file"),
                test_tool_row("nonexistent_tool"),
            ])
        });

        let keys = vec!["file_read".to_string()];
        let (tools, names) = resolve_capabilities_to_tools(&keys, &repo).await.unwrap();

        // Only read_file should resolve (nonexistent_tool not in registry)
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "read_file");
        assert_eq!(names, vec!["read_file"]);
    }
}
