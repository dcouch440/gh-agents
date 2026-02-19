#[cfg(test)]
mod tests {
    use super::super::cycle::would_create_cycle;
    use crate::db::WorkflowStepEdgeRow;
    use uuid::Uuid;

    fn make_edge(from: Uuid, to: Uuid) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
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

    #[test]
    fn cycle_detection_no_edges() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        assert!(!would_create_cycle(a, b, &[]));
    }

    #[test]
    fn cycle_detection_simple_cycle() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        // Existing: A → B. Proposed: B → A. Should detect cycle.
        let edges = vec![make_edge(a, b)];
        assert!(would_create_cycle(b, a, &edges));
    }

    #[test]
    fn cycle_detection_no_cycle() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        // Existing: A → B. Proposed: A → C. No cycle.
        let edges = vec![make_edge(a, b)];
        assert!(!would_create_cycle(a, c, &edges));
    }

    #[test]
    fn cycle_detection_transitive() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        // Existing: A → B → C. Proposed: C → A. Should detect cycle.
        let edges = vec![make_edge(a, b), make_edge(b, c)];
        assert!(would_create_cycle(c, a, &edges));
    }

    #[test]
    fn cycle_detection_self_edge() {
        let a = Uuid::new_v4();
        // Proposed: A → A. Should detect cycle.
        assert!(would_create_cycle(a, a, &[]));
    }

    #[test]
    fn cycle_detection_diamond_no_cycle() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();
        // A → B, A → C, B → D, C → D (diamond). Proposed: A → D. No cycle.
        let edges = vec![
            make_edge(a, b),
            make_edge(a, c),
            make_edge(b, d),
            make_edge(c, d),
        ];
        assert!(!would_create_cycle(a, d, &edges));
    }
}
