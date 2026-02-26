#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::WorkflowStepRow;
    use crate::server::hub::board_serializer::{
        AgentlessChanges, CanvasBounds, CanvasEdge, CanvasNode, CanvasSnapshot, ChangeSignificance,
        FilteredChangeset, GlobalNote, NodeUpdate, ScoredChange,
    };
    use crate::server::services::board::executor::PhaseZeroResult;
    use crate::server::services::board::instruction::{
        build_per_node_instructions, NodeChangeType,
    };

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

    fn make_step_row(id: Uuid, ref_id: &str) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            ref_id: Some(ref_id.to_string()),
            ..Default::default()
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

    // ── Guard cases ───────────────────────────────────────────────────────

    #[test]
    fn should_dispatch_false_returns_empty() {
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

        let result = build_per_node_instructions(&changeset, &empty_phase_zero(), &empty_snapshot());
        assert!(result.is_empty());
    }

    #[test]
    fn empty_meaningful_returns_empty() {
        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![],
            aggregate_score: 0.0,
            should_dispatch: true,
        };

        let result = build_per_node_instructions(&changeset, &empty_phase_zero(), &empty_snapshot());
        assert!(result.is_empty());
    }

    // ── New nodes ─────────────────────────────────────────────────────────

    #[test]
    fn new_node_produces_instruction() {
        let step_id = Uuid::new_v4();

        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![ScoredChange::NewNode {
                node: make_node("n1", "Research competitors"),
                significance: ChangeSignificance::High,
            }],
            aggregate_score: 1.0,
            should_dispatch: true,
        };

        let phase_zero = PhaseZeroResult {
            created_steps: vec![("n1".to_string(), make_step_row(step_id, "workforce-1"))],
            ..empty_phase_zero()
        };

        let result = build_per_node_instructions(&changeset, &phase_zero, &empty_snapshot());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].element_id, "n1");
        assert_eq!(result[0].step_id, step_id);
        assert_eq!(result[0].change_type, NodeChangeType::New);
        assert!(result[0].instruction.contains("Configure this new workflow node."));
        assert!(result[0].instruction.contains("Research competitors"));
    }

    #[test]
    fn new_node_with_annotations() {
        let step_id = Uuid::new_v4();
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
            created_steps: vec![("n1".to_string(), make_step_row(step_id, "workforce-1"))],
            ..empty_phase_zero()
        };

        let result = build_per_node_instructions(&changeset, &phase_zero, &empty_snapshot());

        assert_eq!(result.len(), 1);
        assert!(result[0].instruction.contains("<annotations>"));
        assert!(result[0].instruction.contains("- Focus on pricing"));
        assert!(result[0].instruction.contains("- Q3 data only"));
    }

    #[test]
    fn new_node_with_global_notes() {
        let step_id = Uuid::new_v4();

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

        let phase_zero = PhaseZeroResult {
            created_steps: vec![("n1".to_string(), make_step_row(step_id, "workforce-1"))],
            ..empty_phase_zero()
        };

        let snapshot = CanvasSnapshot {
            nodes: vec![],
            edges: vec![],
            global_notes: vec![GlobalNote {
                element_id: "t1".to_string(),
                text: "Prioritize speed over thoroughness".to_string(),
            }],
        };

        let result = build_per_node_instructions(&changeset, &phase_zero, &snapshot);

        assert_eq!(result.len(), 1);
        assert!(result[0].instruction.contains("<board_notes>"));
        assert!(result[0]
            .instruction
            .contains("Prioritize speed over thoroughness"));
    }

    // ── Updated nodes ─────────────────────────────────────────────────────

    #[test]
    fn updated_node_produces_instruction() {
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
            updated_steps: vec![("n1".to_string(), make_step_row(step_id, "workforce-1"))],
            ..empty_phase_zero()
        };

        let result = build_per_node_instructions(&changeset, &phase_zero, &empty_snapshot());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].element_id, "n1");
        assert_eq!(result[0].step_id, step_id);
        assert_eq!(result[0].change_type, NodeChangeType::Updated);
        assert!(result[0]
            .instruction
            .contains("The user updated this node"));
        assert!(result[0].instruction.contains("Before: \"Analyze data\""));
        assert!(result[0]
            .instruction
            .contains("After: \"Analyze data with year-over-year comparison\""));
    }

    #[test]
    fn updated_node_with_changed_annotations() {
        let step_id = Uuid::new_v4();

        let changeset = FilteredChangeset {
            agentless: empty_agentless(),
            noise: vec![],
            meaningful: vec![ScoredChange::UpdatedNode {
                update: NodeUpdate {
                    element_id: "n1".to_string(),
                    old_text: "Research".to_string(),
                    new_text: "Research competitors".to_string(),
                    old_annotations: vec![],
                    new_annotations: vec!["Focus on pricing".to_string()],
                },
                significance: ChangeSignificance::High,
                token_change_ratio: 0.5,
            }],
            aggregate_score: 1.0,
            should_dispatch: true,
        };

        let phase_zero = PhaseZeroResult {
            updated_steps: vec![("n1".to_string(), make_step_row(step_id, "workforce-1"))],
            ..empty_phase_zero()
        };

        let result = build_per_node_instructions(&changeset, &phase_zero, &empty_snapshot());

        assert_eq!(result.len(), 1);
        assert!(result[0].instruction.contains("<annotations>"));
        assert!(result[0].instruction.contains("- Focus on pricing"));
    }

    // ── Edge-only changesets ──────────────────────────────────────────────

    #[test]
    fn edges_skipped_no_instructions() {
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

        let result = build_per_node_instructions(&changeset, &empty_phase_zero(), &empty_snapshot());
        assert!(result.is_empty());
    }

    // ── Multiple nodes ────────────────────────────────────────────────────

    #[test]
    fn multiple_nodes_produce_multiple_instructions() {
        let step_id_1 = Uuid::new_v4();
        let step_id_2 = Uuid::new_v4();
        let step_id_3 = Uuid::new_v4();

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
                ScoredChange::UpdatedNode {
                    update: NodeUpdate {
                        element_id: "n3".to_string(),
                        old_text: "Validate".to_string(),
                        new_text: "Validate and verify".to_string(),
                        old_annotations: vec![],
                        new_annotations: vec![],
                    },
                    significance: ChangeSignificance::Medium,
                    token_change_ratio: 0.3,
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
            created_steps: vec![
                ("n1".to_string(), make_step_row(step_id_1, "workforce-1")),
                ("n2".to_string(), make_step_row(step_id_2, "workforce-2")),
            ],
            updated_steps: vec![("n3".to_string(), make_step_row(step_id_3, "workforce-3"))],
            ..empty_phase_zero()
        };

        let result = build_per_node_instructions(&changeset, &phase_zero, &empty_snapshot());

        // 3 node instructions (2 new + 1 updated), edge skipped
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].element_id, "n1");
        assert_eq!(result[0].change_type, NodeChangeType::New);
        assert_eq!(result[1].element_id, "n2");
        assert_eq!(result[1].change_type, NodeChangeType::New);
        assert_eq!(result[2].element_id, "n3");
        assert_eq!(result[2].change_type, NodeChangeType::Updated);
    }

    // ── Missing step lookup ───────────────────────────────────────────────

    #[test]
    fn missing_step_lookup_skips_node() {
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

        // No matching step in phase_zero
        let result = build_per_node_instructions(&changeset, &empty_phase_zero(), &empty_snapshot());
        assert!(result.is_empty());
    }

    // ── Sketch data ───────────────────────────────────────────────────────

    #[test]
    fn new_node_with_stroke_encoding() {
        let step_id = Uuid::new_v4();
        let mut node = make_node("n1", "Diagram node");
        node.stroke_encoding = Some("[{\"points\":[[0,0],[10,10]]}]".to_string());

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
            created_steps: vec![("n1".to_string(), make_step_row(step_id, "workforce-1"))],
            ..empty_phase_zero()
        };

        let result = build_per_node_instructions(&changeset, &phase_zero, &empty_snapshot());

        assert_eq!(result.len(), 1);
        assert!(result[0].instruction.contains("<sketch>"));
        assert!(result[0].instruction.contains("points"));
    }
}
