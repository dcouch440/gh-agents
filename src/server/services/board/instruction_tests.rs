#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::server::hub::board_serializer::{
        AgentlessChanges, CanvasBounds, CanvasEdge, CanvasNode, CanvasSnapshot, ChangeSignificance,
        EdgeRewire, FilteredChangeset, GlobalNote, NodeMove, NodeUpdate, ScoredChange,
    };
    use crate::server::services::board::executor::PhaseZeroResult;
    use crate::server::services::board::instruction::format_board_instruction;

    fn empty_snapshot() -> CanvasSnapshot {
        CanvasSnapshot {
            nodes: vec![],
            edges: vec![],
            global_notes: vec![],
        }
    }

    fn empty_agentless() -> AgentlessChanges {
        AgentlessChanges {
            deleted_node_ids: vec![],
            deleted_edge_ids: vec![],
            rewired_edges: vec![],
            moved_nodes: vec![],
        }
    }

    fn empty_phase_zero() -> PhaseZeroResult {
        PhaseZeroResult {
            created_steps: vec![],
            created_edges: vec![],
            deleted_steps: vec![],
            deleted_edges: vec![],
            rewired_edges: vec![],
            moved_steps: vec![],
            updated_steps: vec![],
        }
    }

    fn make_node(element_id: &str, text: &str) -> CanvasNode {
        CanvasNode {
            element_id: element_id.to_string(),
            raw_text: text.to_string(),
            bounds: CanvasBounds {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 100.0,
            },
            annotations: vec![],
            sketch: None,
            stroke_encoding: None,
        }
    }

    // ── should_dispatch / empty cases ────────────────────────────────────

    #[test]
    fn should_dispatch_false_returns_none() {
        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![ScoredChange::NewNode {
                node: make_node("n1", "Research"),
                significance: ChangeSignificance::High,
            }],
            aggregate_score: 1.0,
            should_dispatch: false,
        };

        assert!(
            format_board_instruction(&changeset, &empty_phase_zero(), &empty_snapshot()).is_none()
        );
    }

    #[test]
    fn empty_meaningful_returns_none() {
        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![],
            aggregate_score: 0.0,
            should_dispatch: true,
        };

        assert!(
            format_board_instruction(&changeset, &empty_phase_zero(), &empty_snapshot()).is_none()
        );
    }

    // ── New nodes ────────────────────────────────────────────────────────

    #[test]
    fn new_nodes_instruction() {
        let step_id_1 = Uuid::new_v4();
        let step_id_2 = Uuid::new_v4();

        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![
                ScoredChange::NewNode {
                    node: make_node("n1", "Research competitors"),
                    significance: ChangeSignificance::High,
                },
                ScoredChange::NewNode {
                    node: make_node("n2", "Write report"),
                    significance: ChangeSignificance::High,
                },
            ],
            aggregate_score: 1.0,
            should_dispatch: true,
        };

        let phase_zero = PhaseZeroResult {
            created_steps: vec![
                ("n1".to_string(), step_id_1, "workforce-1".to_string()),
                ("n2".to_string(), step_id_2, "workforce-2".to_string()),
            ],
            ..empty_phase_zero()
        };

        let result = format_board_instruction(&changeset, &phase_zero, &empty_snapshot()).unwrap();

        assert!(result.contains("<new_nodes count=\"2\">"));
        assert!(result.contains("Research competitors"));
        assert!(result.contains("Write report"));
        assert!(result.contains("ref_id=\"workforce-1\""));
        assert!(result.contains("ref_id=\"workforce-2\""));
    }

    // ── Updated nodes ────────────────────────────────────────────────────

    #[test]
    fn updated_nodes_instruction() {
        let step_id = Uuid::new_v4();

        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![ScoredChange::UpdatedNode {
                update: NodeUpdate {
                    element_id: "n1".to_string(),
                    old_text: "Analyze data".to_string(),
                    new_text: "Analyze data with year-over-year comparison".to_string(),
                    old_annotations: vec![],
                    new_annotations: vec![],
                },
                significance: ChangeSignificance::High,
                token_change_ratio: 0.5,
            }],
            aggregate_score: 1.0,
            should_dispatch: true,
        };

        let phase_zero = PhaseZeroResult {
            updated_steps: vec![("n1".to_string(), step_id, "workforce-1".to_string())],
            ..empty_phase_zero()
        };

        let result = format_board_instruction(&changeset, &phase_zero, &empty_snapshot()).unwrap();

        assert!(result.contains("<updated_nodes count=\"1\">"));
        assert!(result.contains("Before: \"Analyze data\""));
        assert!(result.contains("After: \"Analyze data with year-over-year comparison\""));
        assert!(result.contains("ref_id=\"workforce-1\""));
    }

    // ── Mixed changes ────────────────────────────────────────────────────

    #[test]
    fn mixed_changes() {
        let new_step_id = Uuid::new_v4();
        let updated_step_id = Uuid::new_v4();

        let snapshot = CanvasSnapshot {
            nodes: vec![make_node("n1", "Research"), make_node("n2", "Write report")],
            edges: vec![],
            global_notes: vec![],
        };

        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![
                ScoredChange::NewNode {
                    node: make_node("n1", "Research"),
                    significance: ChangeSignificance::High,
                },
                ScoredChange::UpdatedNode {
                    update: NodeUpdate {
                        element_id: "n2".to_string(),
                        old_text: "Write".to_string(),
                        new_text: "Write report".to_string(),
                        old_annotations: vec![],
                        new_annotations: vec![],
                    },
                    significance: ChangeSignificance::Medium,
                    token_change_ratio: 0.15,
                },
                ScoredChange::NewEdge {
                    edge: CanvasEdge {
                        element_id: "e1".to_string(),
                        source_node_id: "n1".to_string(),
                        target_node_id: "n2".to_string(),
                    },
                    significance: ChangeSignificance::Medium,
                },
            ],
            aggregate_score: 1.0,
            should_dispatch: true,
        };

        let phase_zero = PhaseZeroResult {
            created_steps: vec![("n1".to_string(), new_step_id, "workforce-1".to_string())],
            updated_steps: vec![("n2".to_string(), updated_step_id, "workforce-2".to_string())],
            ..empty_phase_zero()
        };

        let result = format_board_instruction(&changeset, &phase_zero, &snapshot).unwrap();

        assert!(result.contains("<new_nodes count=\"1\">"));
        assert!(result.contains("<updated_nodes count=\"1\">"));
        assert!(result.contains("<new_edges count=\"1\">"));
        // Edges use ref_ids for nodes in the changeset
        assert!(result.contains("workforce-1 -> workforce-2"));
    }

    // ── Global notes ─────────────────────────────────────────────────────

    #[test]
    fn with_global_notes() {
        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![ScoredChange::NewNode {
                node: make_node("n1", "Research"),
                significance: ChangeSignificance::High,
            }],
            aggregate_score: 1.0,
            should_dispatch: true,
        };

        let snapshot = CanvasSnapshot {
            nodes: vec![],
            edges: vec![],
            global_notes: vec![GlobalNote {
                element_id: "t1".to_string(),
                text: "Prioritize speed over thoroughness".to_string(),
            }],
        };

        let phase_zero = PhaseZeroResult {
            created_steps: vec![("n1".to_string(), Uuid::new_v4(), "workforce-1".to_string())],
            ..empty_phase_zero()
        };

        let result = format_board_instruction(&changeset, &phase_zero, &snapshot).unwrap();

        assert!(result.contains("<global_notes>"));
        assert!(result.contains("Prioritize speed over thoroughness"));
    }

    // ── Annotations ──────────────────────────────────────────────────────

    #[test]
    fn with_annotations() {
        let mut node = make_node("n1", "Research competitors");
        node.annotations = vec!["Focus on pricing".to_string(), "Q3 data only".to_string()];

        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![ScoredChange::NewNode {
                node,
                significance: ChangeSignificance::High,
            }],
            aggregate_score: 1.0,
            should_dispatch: true,
        };

        let phase_zero = PhaseZeroResult {
            created_steps: vec![("n1".to_string(), Uuid::new_v4(), "workforce-1".to_string())],
            ..empty_phase_zero()
        };

        let result = format_board_instruction(&changeset, &phase_zero, &empty_snapshot()).unwrap();

        assert!(result.contains("Annotations: \"Focus on pricing\", \"Q3 data only\""));
    }

    // ── Agentless summary ────────────────────────────────────────────────

    #[test]
    fn agentless_summary() {
        let changeset = FilteredChangeset {
            agentless: AgentlessChanges {
                deleted_node_ids: vec!["d1".to_string()],
                deleted_edge_ids: vec![],
                rewired_edges: vec![
                    EdgeRewire {
                        element_id: "e1".to_string(),
                        old_source: "a".to_string(),
                        old_target: "b".to_string(),
                        new_source: "a".to_string(),
                        new_target: "c".to_string(),
                    },
                    EdgeRewire {
                        element_id: "e2".to_string(),
                        old_source: "x".to_string(),
                        old_target: "y".to_string(),
                        new_source: "x".to_string(),
                        new_target: "z".to_string(),
                    },
                ],
                moved_nodes: vec![],
            },
            noise: vec![],
            meaningful: vec![ScoredChange::NewNode {
                node: make_node("n1", "New node"),
                significance: ChangeSignificance::High,
            }],
            aggregate_score: 1.0,
            should_dispatch: true,
        };

        let phase_zero = PhaseZeroResult {
            created_steps: vec![("n1".to_string(), Uuid::new_v4(), "workforce-1".to_string())],
            ..empty_phase_zero()
        };

        let result = format_board_instruction(&changeset, &phase_zero, &empty_snapshot()).unwrap();

        assert!(result.contains("<structural_summary>"));
        assert!(result.contains("1 deleted node(s)"));
        assert!(result.contains("2 rewired edge(s)"));
    }

    // ── Edge names resolved ──────────────────────────────────────────────

    #[test]
    fn edge_names_resolved() {
        let snapshot = CanvasSnapshot {
            nodes: vec![make_node("n1", "Research"), make_node("n2", "Write report")],
            edges: vec![],
            global_notes: vec![],
        };

        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![ScoredChange::NewEdge {
                edge: CanvasEdge {
                    element_id: "e1".to_string(),
                    source_node_id: "n1".to_string(),
                    target_node_id: "n2".to_string(),
                },
                significance: ChangeSignificance::Medium,
            }],
            aggregate_score: 0.5,
            should_dispatch: true,
        };

        let result = format_board_instruction(&changeset, &empty_phase_zero(), &snapshot).unwrap();

        assert!(result.contains("Research -> Write report"));
    }

    // ── Moved nodes only (no meaningful → None) ──────────────────────────

    #[test]
    fn moved_nodes_only_no_dispatch() {
        let changeset = FilteredChangeset {
            agentless: AgentlessChanges {
                deleted_node_ids: vec![],
                deleted_edge_ids: vec![],
                rewired_edges: vec![],
                moved_nodes: vec![NodeMove {
                    element_id: "n1".to_string(),
                    old_bounds: CanvasBounds {
                        x: 0.0,
                        y: 0.0,
                        width: 200.0,
                        height: 100.0,
                    },
                    new_bounds: CanvasBounds {
                        x: 100.0,
                        y: 100.0,
                        width: 200.0,
                        height: 100.0,
                    },
                }],
            },
            noise: vec![],
            meaningful: vec![],
            aggregate_score: 0.0,
            should_dispatch: true,
        };

        assert!(
            format_board_instruction(&changeset, &empty_phase_zero(), &empty_snapshot()).is_none()
        );
    }
}
