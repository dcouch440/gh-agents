#[cfg(test)]
mod tests {
    use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
    use uuid::Uuid;

    use super::super::detect_for_each_chains;

    fn make_step(id: Uuid, var_name: Option<&str>, display_order: i32) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id: Uuid::new_v4(),
            agent_id: Some(Uuid::new_v4()),
            prompt_template: "Prompt".into(),
            output_variable_name: var_name.map(|s| s.into()),
            display_order,
            ..Default::default()
        }
    }

    fn make_for_each_step(id: Uuid, var_name: Option<&str>, display_order: i32) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id: Uuid::new_v4(),
            agent_id: Some(Uuid::new_v4()),
            execution_mode: "for_each".into(),
            agent_execution_mode: Some("parallel".into()),
            for_each_ref: Some("items".into()),
            prompt_template: "Process item".into(),
            output_variable_name: var_name.map(|s| s.into()),
            display_order,
            ..Default::default()
        }
    }

    fn make_edge(from: Uuid, to: Uuid) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: from,
            to_step_id: to,
            workflow_id: Uuid::new_v4(),
            ..Default::default()
        }
    }

    #[test]
    fn detect_chains_two_for_each() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let steps = vec![
            make_for_each_step(a, Some("specialists"), 0),
            make_for_each_step(b, Some("reviewers"), 1),
            make_step(c, Some("final"), 2),
        ];
        let edges = vec![make_edge(a, b), make_edge(b, c)];

        let chains = detect_for_each_chains(&steps, &edges);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].step_ids, vec![a, b]);
    }

    #[test]
    fn detect_chains_three_for_each() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();

        let steps = vec![
            make_for_each_step(a, Some("stage1"), 0),
            make_for_each_step(b, Some("stage2"), 1),
            make_for_each_step(c, Some("stage3"), 2),
            make_step(d, Some("final"), 3),
        ];
        let edges = vec![make_edge(a, b), make_edge(b, c), make_edge(c, d)];

        let chains = detect_for_each_chains(&steps, &edges);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].step_ids, vec![a, b, c]);
    }

    #[test]
    fn detect_chains_none_single_for_each() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let steps = vec![
            make_for_each_step(a, Some("items"), 0),
            make_step(b, Some("result"), 1),
        ];
        let edges = vec![make_edge(a, b)];

        let chains = detect_for_each_chains(&steps, &edges);
        assert!(chains.is_empty());
    }

    #[test]
    fn detect_chains_broken_by_single() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let steps = vec![
            make_for_each_step(a, Some("stage1"), 0),
            make_step(b, Some("mid"), 1),
            make_for_each_step(c, Some("stage2"), 2),
        ];
        let edges = vec![make_edge(a, b), make_edge(b, c)];

        let chains = detect_for_each_chains(&steps, &edges);
        assert!(chains.is_empty());
    }

    #[test]
    fn detect_chains_fan_out_breaks() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let steps = vec![
            make_for_each_step(a, Some("source"), 0),
            make_for_each_step(b, Some("branch1"), 1),
            make_for_each_step(c, Some("branch2"), 2),
        ];
        // a fans out to both b and c
        let edges = vec![make_edge(a, b), make_edge(a, c)];

        let chains = detect_for_each_chains(&steps, &edges);
        // a has 2 for-each children, so no chain forms
        assert!(chains.is_empty());
    }

    #[test]
    fn detect_chains_independent() {
        let a1 = Uuid::new_v4();
        let a2 = Uuid::new_v4();
        let b1 = Uuid::new_v4();
        let b2 = Uuid::new_v4();

        let steps = vec![
            make_for_each_step(a1, Some("chain1_s1"), 0),
            make_for_each_step(a2, Some("chain1_s2"), 1),
            make_for_each_step(b1, Some("chain2_s1"), 2),
            make_for_each_step(b2, Some("chain2_s2"), 3),
        ];
        let edges = vec![make_edge(a1, a2), make_edge(b1, b2)];

        let chains = detect_for_each_chains(&steps, &edges);
        assert_eq!(chains.len(), 2);

        // Both chains should have length 2
        for chain in &chains {
            assert_eq!(chain.step_ids.len(), 2);
        }
    }

    #[test]
    fn detect_chains_fan_in_breaks() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let steps = vec![
            make_for_each_step(a, Some("source1"), 0),
            make_for_each_step(b, Some("source2"), 1),
            make_for_each_step(c, Some("merged"), 2),
        ];
        // Both a and b feed into c (fan-in)
        let edges = vec![make_edge(a, c), make_edge(b, c)];

        let chains = detect_for_each_chains(&steps, &edges);
        // c has 2 parents, so no chain forms with c
        assert!(chains.is_empty());
    }
}
