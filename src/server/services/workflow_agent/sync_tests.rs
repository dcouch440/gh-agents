#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use uuid::Uuid;

    use crate::db::WorkflowStepRow;
    use crate::server::services::workflow_agent::sync::{
        auto_layout, diff_edges, diff_nodes, DesiredNode,
    };

    // ── Helpers ────────────────────────────────────────────────────────

    fn make_step(slug: &str, description: &str) -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            agent_id: None,
            execution_mode: "workforce".to_string(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: None,
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
            name: None,
            system_prompt_suffix: None,
            visible: true,
            description: description.to_string(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            child_workflow_id: None,
            ref_id: Some(slug.to_string()),
            pinned: false,
            run_results_summary: String::new(),
            designer_handoff: String::new(),
        }
    }

    fn make_edge(from: Uuid, to: Uuid) -> crate::db::WorkflowStepEdgeRow {
        crate::db::WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: from,
            to_step_id: to,
            from_output_port: None,
            to_input_port: None,
            transform_jsonpath: None,
            condition_type: None,
            condition_value: None,
            edge_label: None,
            workflow_id: Uuid::new_v4(),
        }
    }

    // ── diff_nodes ─────────────────────────────────────────────────────

    #[test]
    fn diff_nodes_first_run_all_creates() {
        let desired = vec![
            DesiredNode {
                slug: "research".into(),
                description: "Do research".into(),
                depends_on: vec![],
            },
            DesiredNode {
                slug: "report".into(),
                description: "Write report".into(),
                depends_on: vec!["research".into()],
            },
        ];
        let current: Vec<WorkflowStepRow> = vec![];

        let diff = diff_nodes(&desired, &current);
        assert_eq!(diff.to_create.len(), 2);
        assert!(diff.to_update.is_empty());
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_nodes_no_changes() {
        let desired = vec![DesiredNode {
            slug: "research".into(),
            description: "Do research".into(),
            depends_on: vec![],
        }];
        let current = vec![make_step("research", "Do research")];

        let diff = diff_nodes(&desired, &current);
        assert!(diff.to_create.is_empty());
        assert!(diff.to_update.is_empty());
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_nodes_description_changed() {
        let desired = vec![DesiredNode {
            slug: "research".into(),
            description: "Updated description".into(),
            depends_on: vec![],
        }];
        let current = vec![make_step("research", "Old description")];

        let diff = diff_nodes(&desired, &current);
        assert!(diff.to_create.is_empty());
        assert_eq!(diff.to_update.len(), 1);
        assert_eq!(diff.to_update[0].1, "research");
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_nodes_remove_missing() {
        let desired: Vec<DesiredNode> = vec![];
        let current = vec![make_step("research", "Old")];

        let diff = diff_nodes(&desired, &current);
        assert!(diff.to_create.is_empty());
        assert!(diff.to_update.is_empty());
        assert_eq!(diff.to_remove.len(), 1);
        assert_eq!(diff.to_remove[0].1, "research");
    }

    #[test]
    fn diff_nodes_mixed_operations() {
        let desired = vec![
            DesiredNode {
                slug: "research".into(),
                description: "Updated".into(),
                depends_on: vec![],
            },
            DesiredNode {
                slug: "new_node".into(),
                description: "New".into(),
                depends_on: vec![],
            },
        ];
        let current = vec![make_step("research", "Old"), make_step("deleted", "Gone")];

        let diff = diff_nodes(&desired, &current);
        assert_eq!(diff.to_create, vec!["new_node"]);
        assert_eq!(diff.to_update.len(), 1);
        assert_eq!(diff.to_update[0].1, "research");
        assert_eq!(diff.to_remove.len(), 1);
        assert_eq!(diff.to_remove[0].1, "deleted");
    }

    #[test]
    fn diff_nodes_ignores_non_workforce() {
        let desired: Vec<DesiredNode> = vec![];
        let mut context_step = make_step("ctx", "context");
        context_step.execution_mode = "context".to_string();

        let diff = diff_nodes(&desired, &[context_step]);
        assert!(diff.to_remove.is_empty()); // should not try to remove non-workforce
    }

    // ── diff_edges ─────────────────────────────────────────────────────

    #[test]
    fn diff_edges_first_run() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        let desired = vec![
            DesiredNode {
                slug: "a".into(),
                description: "A".into(),
                depends_on: vec![],
            },
            DesiredNode {
                slug: "b".into(),
                description: "B".into(),
                depends_on: vec!["a".into()],
            },
        ];

        let slug_to_id: HashMap<String, Uuid> = [("a".into(), step_a), ("b".into(), step_b)].into();
        let workforce_ids: HashSet<Uuid> = [step_a, step_b].into();

        let diff = diff_edges(&desired, &slug_to_id, &[], &workforce_ids);
        assert_eq!(diff.to_add.len(), 1);
        assert_eq!(diff.to_add[0], (step_a, step_b));
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_edges_no_changes() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        let desired = vec![
            DesiredNode {
                slug: "a".into(),
                description: "A".into(),
                depends_on: vec![],
            },
            DesiredNode {
                slug: "b".into(),
                description: "B".into(),
                depends_on: vec!["a".into()],
            },
        ];

        let slug_to_id: HashMap<String, Uuid> = [("a".into(), step_a), ("b".into(), step_b)].into();
        let workforce_ids: HashSet<Uuid> = [step_a, step_b].into();
        let current = vec![make_edge(step_a, step_b)];

        let diff = diff_edges(&desired, &slug_to_id, &current, &workforce_ids);
        assert!(diff.to_add.is_empty());
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn diff_edges_remove_stale() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        let desired = vec![
            DesiredNode {
                slug: "a".into(),
                description: "A".into(),
                depends_on: vec![],
            },
            DesiredNode {
                slug: "b".into(),
                description: "B".into(),
                depends_on: vec![], // no longer depends on a
            },
        ];

        let slug_to_id: HashMap<String, Uuid> = [("a".into(), step_a), ("b".into(), step_b)].into();
        let workforce_ids: HashSet<Uuid> = [step_a, step_b].into();
        let current = vec![make_edge(step_a, step_b)];

        let diff = diff_edges(&desired, &slug_to_id, &current, &workforce_ids);
        assert!(diff.to_add.is_empty());
        assert_eq!(diff.to_remove.len(), 1);
    }

    #[test]
    fn diff_edges_ignores_non_workforce() {
        let step_a = Uuid::new_v4();
        let non_workforce = Uuid::new_v4();

        let desired: Vec<DesiredNode> = vec![DesiredNode {
            slug: "a".into(),
            description: "A".into(),
            depends_on: vec![],
        }];

        let slug_to_id: HashMap<String, Uuid> = [("a".into(), step_a)].into();
        let workforce_ids: HashSet<Uuid> = [step_a].into();
        // Edge from non-workforce step — should be ignored
        let current = vec![make_edge(non_workforce, step_a)];

        let diff = diff_edges(&desired, &slug_to_id, &current, &workforce_ids);
        assert!(diff.to_add.is_empty());
        assert!(diff.to_remove.is_empty());
    }

    // ── auto_layout ────────────────────────────────────────────────────

    #[test]
    fn auto_layout_linear_chain() {
        let mut step_a = make_step("a", "A");
        let mut step_b = make_step("b", "B");
        let mut step_c = make_step("c", "C");
        step_a.id = Uuid::from_u128(1);
        step_b.id = Uuid::from_u128(2);
        step_c.id = Uuid::from_u128(3);

        let steps: Vec<&WorkflowStepRow> = vec![&step_a, &step_b, &step_c];
        let edges = vec![
            make_edge(step_a.id, step_b.id),
            make_edge(step_b.id, step_c.id),
        ];

        let positions = auto_layout(&steps, &edges);
        assert_eq!(positions.len(), 3);

        // a at level 0, b at level 1, c at level 2
        let pos_map: HashMap<Uuid, (f64, f64)> = positions
            .into_iter()
            .map(|(id, x, y)| (id, (x, y)))
            .collect();

        assert!(pos_map[&step_a.id].0 < pos_map[&step_b.id].0);
        assert!(pos_map[&step_b.id].0 < pos_map[&step_c.id].0);
    }

    #[test]
    fn auto_layout_parallel_roots() {
        let mut step_a = make_step("a", "A");
        let mut step_b = make_step("b", "B");
        step_a.id = Uuid::from_u128(1);
        step_b.id = Uuid::from_u128(2);

        let steps: Vec<&WorkflowStepRow> = vec![&step_a, &step_b];
        let edges = vec![];

        let positions = auto_layout(&steps, &edges);
        assert_eq!(positions.len(), 2);

        // Both at level 0, different y
        let pos_map: HashMap<Uuid, (f64, f64)> = positions
            .into_iter()
            .map(|(id, x, y)| (id, (x, y)))
            .collect();

        assert_eq!(pos_map[&step_a.id].0, pos_map[&step_b.id].0); // same x
        assert_ne!(pos_map[&step_a.id].1, pos_map[&step_b.id].1); // different y
    }
}
