#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::TaskAgentRosterRow;
    use crate::server::services::system_node::sync::{diff_agents, diff_edges, DesiredAgent};

    fn desired(name: &str, role: &str, caps: &[&str], deps: &[&str]) -> DesiredAgent {
        use crate::server::services::system_node::normalize_agent_name;
        DesiredAgent {
            slug: normalize_agent_name(name),
            name: name.to_string(),
            role_description: role.to_string(),
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn desired_with_slug(
        slug: &str,
        name: &str,
        role: &str,
        caps: &[&str],
        deps: &[&str],
    ) -> DesiredAgent {
        DesiredAgent {
            slug: slug.to_string(),
            name: name.to_string(),
            role_description: role.to_string(),
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn roster(name: &str, role: &str, caps: &[&str]) -> TaskAgentRosterRow {
        TaskAgentRosterRow {
            id: Uuid::new_v4(),
            name: name.to_string(),
            role_description: role.to_string(),
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
            child_step_id: Some(Uuid::new_v4()),
            ..Default::default()
        }
    }

    // ── Agent diff tests ─────────────────────────────────────────────────

    #[test]
    fn diff_agents_first_run() {
        let desired = vec![
            desired("Scanner", "Scan code.", &[], &[]),
            desired("Analyzer", "Analyze.", &[], &["scanner"]),
            desired("Reporter", "Report.", &[], &["analyzer"]),
        ];
        let current: Vec<TaskAgentRosterRow> = vec![];

        let diff = diff_agents(&desired, &current);

        assert_eq!(diff.to_create.len(), 3);
        assert!(diff.to_update.is_empty());
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_agents_no_change() {
        let desired = vec![
            desired("Scanner", "Scan code.", &[], &[]),
            desired("Analyzer", "Analyze.", &[], &["scanner"]),
        ];
        let current = vec![
            roster("Scanner", "Scan code.", &[]),
            roster("Analyzer", "Analyze.", &[]),
        ];

        let diff = diff_agents(&desired, &current);

        assert!(diff.to_create.is_empty());
        assert!(diff.to_update.is_empty());
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_agents_add_one() {
        let desired = vec![
            desired("Scanner", "Scan.", &[], &[]),
            desired("Analyzer", "Analyze.", &[], &["scanner"]),
            desired("Reporter", "Report.", &[], &["analyzer"]),
        ];
        let current = vec![
            roster("Scanner", "Scan.", &[]),
            roster("Analyzer", "Analyze.", &[]),
        ];

        let diff = diff_agents(&desired, &current);

        assert_eq!(diff.to_create, vec!["Reporter"]);
        assert!(diff.to_update.is_empty());
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_agents_remove_one() {
        let desired = vec![
            desired("Scanner", "Scan.", &[], &[]),
            desired("Analyzer", "Analyze.", &[], &["scanner"]),
        ];
        let current = vec![
            roster("Scanner", "Scan.", &[]),
            roster("Analyzer", "Analyze.", &[]),
            roster("Reporter", "Report.", &[]),
        ];

        let diff = diff_agents(&desired, &current);

        assert!(diff.to_create.is_empty());
        assert!(diff.to_update.is_empty());
        assert_eq!(diff.to_remove.len(), 1);
        assert_eq!(diff.to_remove[0].1, "Reporter");
    }

    #[test]
    fn diff_agents_update_role() {
        let desired = vec![desired(
            "Scanner",
            "Security vulnerability scanner.",
            &[],
            &[],
        )];
        let current = vec![roster("Scanner", "Scan code.", &[])];

        let diff = diff_agents(&desired, &current);

        assert!(diff.to_create.is_empty());
        assert_eq!(diff.to_update.len(), 1);
        assert_eq!(diff.to_update[0].1, "Scanner");
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_agents_update_capabilities() {
        let desired = vec![desired("Scanner", "Scan.", &["database_query"], &[])];
        let current = vec![roster("Scanner", "Scan.", &[])];

        let diff = diff_agents(&desired, &current);

        assert_eq!(diff.to_update.len(), 1);
        assert_eq!(diff.to_update[0].1, "Scanner");
    }

    #[test]
    fn diff_agents_name_normalization() {
        let desired = vec![desired("Web Researcher", "Research.", &[], &[])];
        let current = vec![roster("web_researcher", "Research.", &[])];

        let diff = diff_agents(&desired, &current);

        // Should match despite different casing/separators
        assert!(diff.to_create.is_empty());
        assert!(diff.to_update.is_empty());
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_agents_mixed_operations() {
        let desired = vec![
            desired("Scanner", "Updated scanner.", &["web_search"], &[]),
            desired("Writer", "Write reports.", &[], &["scanner"]),
        ];
        let current = vec![
            roster("Scanner", "Old scanner.", &[]),
            roster("Analyzer", "Analyze.", &[]),
        ];

        let diff = diff_agents(&desired, &current);

        assert_eq!(diff.to_create, vec!["Writer"]);
        assert_eq!(diff.to_update.len(), 1);
        assert_eq!(diff.to_update[0].1, "Scanner");
        assert_eq!(diff.to_remove.len(), 1);
        assert_eq!(diff.to_remove[0].1, "Analyzer");
    }

    // ── Edge diff tests ──────────────────────────────────────────────────

    #[test]
    fn diff_edges_first_run() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();
        let step_c = Uuid::new_v4();

        let desired = vec![
            desired("Scanner", "Scan.", &[], &[]),
            desired("Analyzer", "Analyze.", &[], &["scanner"]),
            desired("Reporter", "Report.", &[], &["analyzer"]),
        ];

        let mut name_to_step = std::collections::HashMap::new();
        name_to_step.insert("scanner".to_string(), step_a);
        name_to_step.insert("analyzer".to_string(), step_b);
        name_to_step.insert("reporter".to_string(), step_c);

        let agent_step_ids: std::collections::HashSet<Uuid> =
            [step_a, step_b, step_c].into_iter().collect();

        let diff = diff_edges(&desired, &name_to_step, &[], &agent_step_ids);

        assert_eq!(diff.to_add.len(), 2);
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_edges_remove_stale() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        // Desired: no dependencies
        let desired = vec![
            desired("Scanner", "Scan.", &[], &[]),
            desired("Analyzer", "Analyze.", &[], &[]),
        ];

        let mut name_to_step = std::collections::HashMap::new();
        name_to_step.insert("scanner".to_string(), step_a);
        name_to_step.insert("analyzer".to_string(), step_b);

        let agent_step_ids: std::collections::HashSet<Uuid> =
            [step_a, step_b].into_iter().collect();

        // Current: one edge exists
        let current_edges = vec![crate::db::WorkflowStepEdgeRow {
            from_step_id: step_a,
            to_step_id: step_b,
            ..Default::default()
        }];

        let diff = diff_edges(&desired, &name_to_step, &current_edges, &agent_step_ids);

        assert!(diff.to_add.is_empty());
        assert_eq!(diff.to_remove.len(), 1);
    }

    #[test]
    fn diff_edges_no_change() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        let desired = vec![
            desired("Scanner", "Scan.", &[], &[]),
            desired("Analyzer", "Analyze.", &[], &["scanner"]),
        ];

        let mut name_to_step = std::collections::HashMap::new();
        name_to_step.insert("scanner".to_string(), step_a);
        name_to_step.insert("analyzer".to_string(), step_b);

        let agent_step_ids: std::collections::HashSet<Uuid> =
            [step_a, step_b].into_iter().collect();

        let current_edges = vec![crate::db::WorkflowStepEdgeRow {
            from_step_id: step_a,
            to_step_id: step_b,
            ..Default::default()
        }];

        let diff = diff_edges(&desired, &name_to_step, &current_edges, &agent_step_ids);

        assert!(diff.to_add.is_empty());
        assert!(diff.to_remove.is_empty());
    }

    /// Regression: depends_on slugs differ from agent display names.
    ///
    /// topology: { "brainstormer": { depends_on: [] }, "curator": { depends_on: ["brainstormer"] } }
    /// agents/brainstormer.json: { name: "Idea Brainstormer" }
    /// agents/curator.json: { name: "Idea Curator" }
    ///
    /// name_to_step is keyed by normalized display name ("ideabrainstormer"),
    /// but depends_on contains the slug ("brainstormer"). The slug→step lookup
    /// must bridge this gap.
    #[test]
    fn diff_edges_slug_differs_from_display_name() {
        let step_brainstormer = Uuid::new_v4();
        let step_curator = Uuid::new_v4();

        let desired = vec![
            desired_with_slug("brainstormer", "Idea Brainstormer", "Brainstorm.", &[], &[]),
            desired_with_slug("curator", "Idea Curator", "Curate.", &[], &["brainstormer"]),
        ];

        // name_to_step is keyed by normalized display name (how sync_to_db builds it)
        let mut name_to_step = std::collections::HashMap::new();
        name_to_step.insert("ideabrainstormer".to_string(), step_brainstormer);
        name_to_step.insert("ideacurator".to_string(), step_curator);

        let agent_step_ids: std::collections::HashSet<Uuid> =
            [step_brainstormer, step_curator].into_iter().collect();

        let diff = diff_edges(&desired, &name_to_step, &[], &agent_step_ids);

        assert_eq!(
            diff.to_add.len(),
            1,
            "should create brainstormer→curator edge"
        );
        assert_eq!(diff.to_add[0], (step_brainstormer, step_curator));
        assert!(diff.to_remove.is_empty());
    }
}
