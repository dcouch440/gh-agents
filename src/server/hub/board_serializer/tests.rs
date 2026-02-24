#[cfg(test)]
mod tests {
    use crate::server::hub::board_serializer::types::*;
    use crate::server::hub::board_serializer::{
        classify_board, classify_board_with_threshold, diff_snapshots,
    };

    // ========================================================================
    // Factory helpers
    // ========================================================================

    /// Create a rectangle + bound text element pair (a node candidate).
    fn make_rect(id: &str, x: f64, y: f64, w: f64, h: f64, text: &str) -> Vec<ExcalidrawElement> {
        let text_id = format!("{id}_text");
        vec![
            ExcalidrawElement::Rectangle(RectangleElement {
                id: id.to_string(),
                x,
                y,
                width: w,
                height: h,
                is_deleted: false,
                bound_elements: vec![BoundElementRef {
                    id: text_id.clone(),
                    kind: "text".to_string(),
                }],
            }),
            ExcalidrawElement::Text(TextElement {
                id: text_id,
                x: x + 10.0,
                y: y + 10.0,
                width: w - 20.0,
                height: h - 20.0,
                is_deleted: false,
                text: text.to_string(),
                container_id: Some(id.to_string()),
            }),
        ]
    }

    /// Create a bare rectangle with no bound text (not a node candidate).
    fn make_bare_rect(id: &str, x: f64, y: f64, w: f64, h: f64) -> ExcalidrawElement {
        ExcalidrawElement::Rectangle(RectangleElement {
            id: id.to_string(),
            x,
            y,
            width: w,
            height: h,
            is_deleted: false,
            bound_elements: vec![],
        })
    }

    /// Create an arrow connecting two elements.
    fn make_arrow(id: &str, from_id: &str, to_id: &str) -> ExcalidrawElement {
        ExcalidrawElement::Arrow(ArrowElement {
            id: id.to_string(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 0.0,
            is_deleted: false,
            start_binding: Some(ArrowBinding {
                element_id: from_id.to_string(),
            }),
            end_binding: Some(ArrowBinding {
                element_id: to_id.to_string(),
            }),
        })
    }

    /// Create an arrow with only one binding (dangling).
    fn make_dangling_arrow(id: &str, from_id: &str) -> ExcalidrawElement {
        ExcalidrawElement::Arrow(ArrowElement {
            id: id.to_string(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 0.0,
            is_deleted: false,
            start_binding: Some(ArrowBinding {
                element_id: from_id.to_string(),
            }),
            end_binding: None,
        })
    }

    /// Create a free-floating text element (not bound to any shape).
    fn make_text(id: &str, x: f64, y: f64, content: &str) -> ExcalidrawElement {
        ExcalidrawElement::Text(TextElement {
            id: id.to_string(),
            x,
            y,
            width: 100.0,
            height: 20.0,
            is_deleted: false,
            text: content.to_string(),
            container_id: None,
        })
    }

    /// Create a deleted rectangle + text pair (should be skipped).
    fn make_deleted_rect(id: &str) -> Vec<ExcalidrawElement> {
        let text_id = format!("{id}_text");
        vec![
            ExcalidrawElement::Rectangle(RectangleElement {
                id: id.to_string(),
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 100.0,
                is_deleted: true,
                bound_elements: vec![BoundElementRef {
                    id: text_id.clone(),
                    kind: "text".to_string(),
                }],
            }),
            ExcalidrawElement::Text(TextElement {
                id: text_id,
                x: 10.0,
                y: 10.0,
                width: 180.0,
                height: 80.0,
                is_deleted: true,
                text: "deleted node".to_string(),
                container_id: Some(id.to_string()),
            }),
        ]
    }

    /// Build a simple CanvasSnapshot for diff testing.
    fn make_snapshot(
        nodes: Vec<CanvasNode>,
        edges: Vec<CanvasEdge>,
        global_notes: Vec<GlobalNote>,
    ) -> CanvasSnapshot {
        CanvasSnapshot {
            nodes,
            edges,
            global_notes,
        }
    }

    fn make_canvas_node(id: &str, text: &str, x: f64, y: f64) -> CanvasNode {
        CanvasNode {
            element_id: id.to_string(),
            raw_text: text.to_string(),
            bounds: CanvasBounds {
                x,
                y,
                width: 200.0,
                height: 100.0,
            },
            annotations: vec![],
            sketch: None,
        }
    }

    /// Create a freedraw element with absolute base position and relative points.
    fn make_freedraw(
        id: &str,
        base_x: f64,
        base_y: f64,
        points: Vec<Vec<f64>>,
    ) -> ExcalidrawElement {
        ExcalidrawElement::Freedraw(FreedrawElement {
            id: id.to_string(),
            x: base_x,
            y: base_y,
            is_deleted: false,
            points,
        })
    }

    /// Create a line element with absolute base position and relative points.
    fn make_line(id: &str, base_x: f64, base_y: f64, points: Vec<Vec<f64>>) -> ExcalidrawElement {
        ExcalidrawElement::Line(LineElement {
            id: id.to_string(),
            x: base_x,
            y: base_y,
            is_deleted: false,
            points,
        })
    }

    fn make_canvas_edge(id: &str, source: &str, target: &str) -> CanvasEdge {
        CanvasEdge {
            element_id: id.to_string(),
            source_node_id: source.to_string(),
            target_node_id: target.to_string(),
        }
    }

    // ========================================================================
    // Classification — Node candidates
    // ========================================================================

    #[test]
    fn classify_rect_with_text() {
        let elements = make_rect(
            "r1",
            0.0,
            0.0,
            200.0,
            100.0,
            "Document Collection\nWorkforce",
        );
        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 1);
        assert_eq!(snapshot.nodes[0].element_id, "r1");
        assert_eq!(snapshot.nodes[0].raw_text, "Document Collection\nWorkforce");
        assert_eq!(snapshot.nodes[0].bounds.x, 0.0);
        assert_eq!(snapshot.nodes[0].bounds.width, 200.0);
    }

    #[test]
    fn classify_rect_without_text() {
        let elements = vec![make_bare_rect("r1", 0.0, 0.0, 200.0, 100.0)];
        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 0);
    }

    #[test]
    fn classify_deleted_elements_skipped() {
        let elements = make_deleted_rect("r1");
        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 0);
        assert_eq!(snapshot.edges.len(), 0);
    }

    #[test]
    fn classify_unknown_elements_skipped() {
        // ExcalidrawElement::Other represents ellipse, diamond, image, etc.
        let elements = vec![ExcalidrawElement::Other];
        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 0);
        assert_eq!(snapshot.edges.len(), 0);
        assert!(snapshot.global_notes.is_empty());
    }

    #[test]
    fn classify_text_bound_to_node_not_unbound() {
        // Text with container_id pointing to a node should NOT appear as unbound text
        let elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "My Node");
        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 1);
        assert!(snapshot.global_notes.is_empty());
        // The text is part of the node, not a separate annotation
        assert!(snapshot.nodes[0].annotations.is_empty());
    }

    #[test]
    fn classify_multiple_nodes() {
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Node A");
        elements.extend(make_rect("r2", 300.0, 0.0, 200.0, 100.0, "Node B"));

        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 2);
        let ids: Vec<&str> = snapshot
            .nodes
            .iter()
            .map(|n| n.element_id.as_str())
            .collect();
        assert!(ids.contains(&"r1"));
        assert!(ids.contains(&"r2"));
    }

    // ========================================================================
    // Classification — Edges
    // ========================================================================

    #[test]
    fn classify_arrow_both_bindings() {
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Source");
        elements.extend(make_rect("r2", 300.0, 0.0, 200.0, 100.0, "Target"));
        elements.push(make_arrow("a1", "r1", "r2"));

        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.edges.len(), 1);
        assert_eq!(snapshot.edges[0].element_id, "a1");
        assert_eq!(snapshot.edges[0].source_node_id, "r1");
        assert_eq!(snapshot.edges[0].target_node_id, "r2");
    }

    #[test]
    fn classify_arrow_one_binding() {
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Source");
        elements.push(make_dangling_arrow("a1", "r1"));

        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.edges.len(), 0);
    }

    #[test]
    fn classify_arrow_to_non_node() {
        // Arrow from a node to a bare rectangle (no text) — not an edge
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Source");
        elements.push(make_bare_rect("r2", 300.0, 0.0, 200.0, 100.0));
        elements.push(make_arrow("a1", "r1", "r2"));

        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.edges.len(), 0);
    }

    #[test]
    fn classify_deleted_arrow_skipped() {
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Source");
        elements.extend(make_rect("r2", 300.0, 0.0, 200.0, 100.0, "Target"));
        elements.push(ExcalidrawElement::Arrow(ArrowElement {
            id: "a1".to_string(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 0.0,
            is_deleted: true,
            start_binding: Some(ArrowBinding {
                element_id: "r1".to_string(),
            }),
            end_binding: Some(ArrowBinding {
                element_id: "r2".to_string(),
            }),
        }));

        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.edges.len(), 0);
    }

    // ========================================================================
    // Classification — Unbound text
    // ========================================================================

    #[test]
    fn classify_unbound_text() {
        let elements = vec![make_text("t1", 500.0, 500.0, "A floating note")];
        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 0);
        assert_eq!(snapshot.global_notes.len(), 1);
        assert_eq!(snapshot.global_notes[0].text, "A floating note");
    }

    #[test]
    fn classify_empty_text_skipped() {
        let elements = vec![make_text("t1", 0.0, 0.0, "")];
        let snapshot = classify_board(&elements);

        assert!(snapshot.global_notes.is_empty());
    }

    // ========================================================================
    // Annotation resolution
    // ========================================================================

    #[test]
    fn annotation_within_threshold() {
        // Node at (0, 0), text at (250, 50) — center at (300, 60)
        // Distance from (300, 60) to node AABB (0,0)-(200,100) = 100px
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "My Node");
        elements.push(make_text("t1", 250.0, 50.0, "annotation text"));

        let snapshot = classify_board_with_threshold(&elements, 101.0);

        assert_eq!(snapshot.nodes.len(), 1);
        assert_eq!(snapshot.nodes[0].annotations.len(), 1);
        assert_eq!(snapshot.nodes[0].annotations[0], "annotation text");
        assert!(snapshot.global_notes.is_empty());
    }

    #[test]
    fn annotation_beyond_threshold() {
        // Node at (0, 0), text far away at (1000, 1000)
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "My Node");
        elements.push(make_text("t1", 1000.0, 1000.0, "far away note"));

        let snapshot = classify_board_with_threshold(&elements, 100.0);

        assert_eq!(snapshot.nodes[0].annotations.len(), 0);
        assert_eq!(snapshot.global_notes.len(), 1);
        assert_eq!(snapshot.global_notes[0].text, "far away note");
    }

    #[test]
    fn annotation_closest_node_wins() {
        // Two nodes, text between them but closer to the second
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Node A");
        elements.extend(make_rect("r2", 400.0, 0.0, 200.0, 100.0, "Node B"));
        // Text at (350, 40) — center at (400, 50), which is right at r2's edge
        elements.push(make_text("t1", 350.0, 40.0, "closer to B"));

        let snapshot = classify_board_with_threshold(&elements, 200.0);

        let node_a = snapshot
            .nodes
            .iter()
            .find(|n| n.element_id == "r1")
            .unwrap();
        let node_b = snapshot
            .nodes
            .iter()
            .find(|n| n.element_id == "r2")
            .unwrap();

        assert!(node_a.annotations.is_empty());
        assert_eq!(node_b.annotations.len(), 1);
        assert_eq!(node_b.annotations[0], "closer to B");
    }

    #[test]
    fn annotation_inside_bounds() {
        // Text center inside the node's AABB — distance 0, definitely an annotation
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "My Node");
        elements.push(make_text("t1", 50.0, 30.0, "inside note"));

        let snapshot = classify_board_with_threshold(&elements, 1.0);

        assert_eq!(snapshot.nodes[0].annotations.len(), 1);
        assert_eq!(snapshot.nodes[0].annotations[0], "inside note");
    }

    #[test]
    fn annotation_empty_input() {
        let elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "My Node");
        let snapshot = classify_board(&elements);

        assert!(snapshot.nodes[0].annotations.is_empty());
        assert!(snapshot.global_notes.is_empty());
    }

    #[test]
    fn multiple_annotations_one_node() {
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "My Node");
        elements.push(make_text("t1", 210.0, 0.0, "note 1"));
        elements.push(make_text("t2", 210.0, 30.0, "note 2"));

        let snapshot = classify_board_with_threshold(&elements, 100.0);

        assert_eq!(snapshot.nodes[0].annotations.len(), 2);
        assert!(snapshot.nodes[0]
            .annotations
            .contains(&"note 1".to_string()));
        assert!(snapshot.nodes[0]
            .annotations
            .contains(&"note 2".to_string()));
    }

    // ========================================================================
    // Diffing — Nodes
    // ========================================================================

    #[test]
    fn diff_new_node() {
        let previous = make_snapshot(vec![], vec![], vec![]);
        let current = make_snapshot(
            vec![make_canvas_node("n1", "New Node", 0.0, 0.0)],
            vec![],
            vec![],
        );

        let changeset = diff_snapshots(&previous, &current);

        assert_eq!(changeset.new_nodes.len(), 1);
        assert_eq!(changeset.new_nodes[0].element_id, "n1");
        assert!(changeset.deleted_node_ids.is_empty());
        assert!(changeset.updated_nodes.is_empty());
        assert!(changeset.moved_nodes.is_empty());
    }

    #[test]
    fn diff_deleted_node() {
        let previous = make_snapshot(
            vec![make_canvas_node("n1", "Old Node", 0.0, 0.0)],
            vec![],
            vec![],
        );
        let current = make_snapshot(vec![], vec![], vec![]);

        let changeset = diff_snapshots(&previous, &current);

        assert_eq!(changeset.deleted_node_ids.len(), 1);
        assert_eq!(changeset.deleted_node_ids[0], "n1");
        assert!(changeset.new_nodes.is_empty());
    }

    #[test]
    fn diff_updated_text() {
        let previous = make_snapshot(
            vec![make_canvas_node("n1", "Old text", 0.0, 0.0)],
            vec![],
            vec![],
        );
        let current = make_snapshot(
            vec![make_canvas_node("n1", "New text", 0.0, 0.0)],
            vec![],
            vec![],
        );

        let changeset = diff_snapshots(&previous, &current);

        assert_eq!(changeset.updated_nodes.len(), 1);
        assert_eq!(changeset.updated_nodes[0].element_id, "n1");
        assert_eq!(changeset.updated_nodes[0].old_text, "Old text");
        assert_eq!(changeset.updated_nodes[0].new_text, "New text");
        assert!(changeset.new_nodes.is_empty());
        assert!(changeset.moved_nodes.is_empty());
    }

    #[test]
    fn diff_updated_annotations() {
        let mut prev_node = make_canvas_node("n1", "Same text", 0.0, 0.0);
        prev_node.annotations = vec!["old note".to_string()];

        let mut curr_node = make_canvas_node("n1", "Same text", 0.0, 0.0);
        curr_node.annotations = vec!["new note".to_string()];

        let previous = make_snapshot(vec![prev_node], vec![], vec![]);
        let current = make_snapshot(vec![curr_node], vec![], vec![]);

        let changeset = diff_snapshots(&previous, &current);

        assert_eq!(changeset.updated_nodes.len(), 1);
        assert_eq!(changeset.updated_nodes[0].old_annotations, vec!["old note"]);
        assert_eq!(changeset.updated_nodes[0].new_annotations, vec!["new note"]);
    }

    #[test]
    fn diff_moved_node() {
        let previous = make_snapshot(
            vec![make_canvas_node("n1", "Same text", 0.0, 0.0)],
            vec![],
            vec![],
        );
        let current = make_snapshot(
            vec![make_canvas_node("n1", "Same text", 100.0, 200.0)],
            vec![],
            vec![],
        );

        let changeset = diff_snapshots(&previous, &current);

        assert!(changeset.updated_nodes.is_empty());
        assert_eq!(changeset.moved_nodes.len(), 1);
        assert_eq!(changeset.moved_nodes[0].element_id, "n1");
        assert_eq!(changeset.moved_nodes[0].old_bounds.x, 0.0);
        assert_eq!(changeset.moved_nodes[0].new_bounds.x, 100.0);
    }

    #[test]
    fn diff_no_changes() {
        let node = make_canvas_node("n1", "Same", 0.0, 0.0);
        let edge = make_canvas_edge("e1", "n1", "n2");

        let snapshot = make_snapshot(vec![node.clone()], vec![edge.clone()], vec![]);

        let changeset = diff_snapshots(&snapshot, &snapshot);

        assert!(changeset.new_nodes.is_empty());
        assert!(changeset.updated_nodes.is_empty());
        assert!(changeset.deleted_node_ids.is_empty());
        assert!(changeset.moved_nodes.is_empty());
        assert!(changeset.new_edges.is_empty());
        assert!(changeset.deleted_edge_ids.is_empty());
        assert!(changeset.rewired_edges.is_empty());
    }

    // ========================================================================
    // Diffing — Edges
    // ========================================================================

    #[test]
    fn diff_new_edge() {
        let previous = make_snapshot(vec![], vec![], vec![]);
        let current = make_snapshot(vec![], vec![make_canvas_edge("e1", "n1", "n2")], vec![]);

        let changeset = diff_snapshots(&previous, &current);

        assert_eq!(changeset.new_edges.len(), 1);
        assert_eq!(changeset.new_edges[0].element_id, "e1");
    }

    #[test]
    fn diff_deleted_edge() {
        let previous = make_snapshot(vec![], vec![make_canvas_edge("e1", "n1", "n2")], vec![]);
        let current = make_snapshot(vec![], vec![], vec![]);

        let changeset = diff_snapshots(&previous, &current);

        assert_eq!(changeset.deleted_edge_ids.len(), 1);
        assert_eq!(changeset.deleted_edge_ids[0], "e1");
    }

    #[test]
    fn diff_rewired_edge() {
        let previous = make_snapshot(vec![], vec![make_canvas_edge("e1", "n1", "n2")], vec![]);
        let current = make_snapshot(vec![], vec![make_canvas_edge("e1", "n1", "n3")], vec![]);

        let changeset = diff_snapshots(&previous, &current);

        assert!(changeset.new_edges.is_empty());
        assert!(changeset.deleted_edge_ids.is_empty());
        assert_eq!(changeset.rewired_edges.len(), 1);
        assert_eq!(changeset.rewired_edges[0].element_id, "e1");
        assert_eq!(changeset.rewired_edges[0].old_target, "n2");
        assert_eq!(changeset.rewired_edges[0].new_target, "n3");
    }

    // ========================================================================
    // Diffing — Mixed changes
    // ========================================================================

    #[test]
    fn diff_mixed_changes() {
        let previous = make_snapshot(
            vec![
                make_canvas_node("n1", "Stays", 0.0, 0.0),
                make_canvas_node("n2", "Gets deleted", 200.0, 0.0),
                make_canvas_node("n3", "Gets updated", 400.0, 0.0),
            ],
            vec![make_canvas_edge("e1", "n1", "n2")],
            vec![],
        );

        let current = make_snapshot(
            vec![
                make_canvas_node("n1", "Stays", 0.0, 0.0),
                make_canvas_node("n3", "Updated text", 400.0, 0.0),
                make_canvas_node("n4", "Brand new", 600.0, 0.0),
            ],
            vec![make_canvas_edge("e2", "n1", "n3")],
            vec![],
        );

        let changeset = diff_snapshots(&previous, &current);

        // n4 is new
        assert_eq!(changeset.new_nodes.len(), 1);
        assert_eq!(changeset.new_nodes[0].element_id, "n4");

        // n2 is deleted
        assert_eq!(changeset.deleted_node_ids.len(), 1);
        assert_eq!(changeset.deleted_node_ids[0], "n2");

        // n3 is updated
        assert_eq!(changeset.updated_nodes.len(), 1);
        assert_eq!(changeset.updated_nodes[0].element_id, "n3");
        assert_eq!(changeset.updated_nodes[0].old_text, "Gets updated");
        assert_eq!(changeset.updated_nodes[0].new_text, "Updated text");

        // e1 is deleted, e2 is new
        assert_eq!(changeset.deleted_edge_ids.len(), 1);
        assert_eq!(changeset.deleted_edge_ids[0], "e1");
        assert_eq!(changeset.new_edges.len(), 1);
        assert_eq!(changeset.new_edges[0].element_id, "e2");
    }

    // ========================================================================
    // Integration — Full pipeline
    // ========================================================================

    #[test]
    fn full_pipeline_simple() {
        // Two nodes connected by an arrow, plus one annotation
        let mut elements = make_rect(
            "r1",
            0.0,
            0.0,
            200.0,
            100.0,
            "Document Collection\nWorkforce\n\nCollect documents from the database.",
        );
        elements.extend(make_rect(
            "r2",
            400.0,
            0.0,
            200.0,
            100.0,
            "Analysis Report\nSingle\n\nAnalyze the documents.",
        ));
        elements.push(make_arrow("a1", "r1", "r2"));
        elements.push(make_text("t1", 210.0, 40.0, "Handle pagination"));

        let snapshot = classify_board_with_threshold(&elements, 100.0);

        // Two nodes
        assert_eq!(snapshot.nodes.len(), 2);

        // One edge
        assert_eq!(snapshot.edges.len(), 1);
        assert_eq!(snapshot.edges[0].source_node_id, "r1");
        assert_eq!(snapshot.edges[0].target_node_id, "r2");

        // Annotation assigned to the nearest node
        let annotated_node = snapshot.nodes.iter().find(|n| !n.annotations.is_empty());
        assert!(annotated_node.is_some());
        assert!(annotated_node
            .unwrap()
            .annotations
            .contains(&"Handle pagination".to_string()));
    }

    #[test]
    fn full_pipeline_with_noise() {
        // Nodes + arrow + Other + deleted rect + bare rect
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Real Node");
        elements.extend(make_rect("r2", 300.0, 0.0, 200.0, 100.0, "Another Node"));
        elements.push(make_arrow("a1", "r1", "r2"));
        elements.push(ExcalidrawElement::Other); // ellipse / diamond / etc.
        elements.extend(make_deleted_rect("r3")); // deleted node
        elements.push(make_bare_rect("r4", 600.0, 0.0, 200.0, 100.0)); // no text

        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.edges.len(), 1);
        // No noise leaked through
        let all_ids: Vec<&str> = snapshot
            .nodes
            .iter()
            .map(|n| n.element_id.as_str())
            .collect();
        assert!(!all_ids.contains(&"r3"));
        assert!(!all_ids.contains(&"r4"));
    }

    #[test]
    fn full_pipeline_diff() {
        // First submit: two nodes
        let mut elements_v1 = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Research");
        elements_v1.extend(make_rect("r2", 300.0, 0.0, 200.0, 100.0, "Write"));
        elements_v1.push(make_arrow("a1", "r1", "r2"));

        let snapshot_v1 = classify_board(&elements_v1);

        // Second submit: changed r1 text, added r3, deleted r2
        let mut elements_v2 = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Research Competitors");
        elements_v2.extend(make_rect("r3", 300.0, 0.0, 200.0, 100.0, "Validate"));
        elements_v2.push(make_arrow("a1", "r1", "r3"));

        let snapshot_v2 = classify_board(&elements_v2);

        let changeset = diff_snapshots(&snapshot_v1, &snapshot_v2);

        // r1 text changed → updated
        assert_eq!(changeset.updated_nodes.len(), 1);
        assert_eq!(changeset.updated_nodes[0].element_id, "r1");
        assert_eq!(changeset.updated_nodes[0].old_text, "Research");
        assert_eq!(changeset.updated_nodes[0].new_text, "Research Competitors");

        // r2 deleted
        assert_eq!(changeset.deleted_node_ids, vec!["r2"]);

        // r3 new
        assert_eq!(changeset.new_nodes.len(), 1);
        assert_eq!(changeset.new_nodes[0].element_id, "r3");

        // a1 rewired (r2 → r3)
        assert_eq!(changeset.rewired_edges.len(), 1);
        assert_eq!(changeset.rewired_edges[0].old_target, "r2");
        assert_eq!(changeset.rewired_edges[0].new_target, "r3");
    }

    // ========================================================================
    // Rasterizer — Unit tests
    // ========================================================================

    use crate::server::hub::board_serializer::rasterize::rasterize_strokes;

    #[test]
    fn rasterize_horizontal_line() {
        // Stroke from left edge to right edge, at vertical midpoint
        let bounds = CanvasBounds {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let stroke = vec![[0.0, 50.0], [100.0, 50.0]];
        let result = rasterize_strokes(&[stroke], &bounds, 10, 10).unwrap();
        let lines: Vec<&str> = result.lines().collect();

        // Row 5 (midpoint) should be all filled
        assert!(
            lines[5].chars().all(|c| c == '█'),
            "middle row should be fully filled"
        );
        // Row 0 should be all empty
        assert!(
            lines[0].chars().all(|c| c == '·'),
            "top row should be empty"
        );
    }

    #[test]
    fn rasterize_vertical_line() {
        // Stroke from top to bottom at horizontal midpoint
        let bounds = CanvasBounds {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let stroke = vec![[50.0, 0.0], [50.0, 100.0]];
        let result = rasterize_strokes(&[stroke], &bounds, 10, 10).unwrap();
        let lines: Vec<&str> = result.lines().collect();

        // Column 5 (midpoint) should be filled in every row
        for line in &lines {
            let chars: Vec<char> = line.chars().collect();
            assert_eq!(chars[5], '█', "middle column should be filled in every row");
        }
    }

    #[test]
    fn rasterize_diagonal_line() {
        let bounds = CanvasBounds {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let stroke = vec![[0.0, 0.0], [100.0, 100.0]];
        let result = rasterize_strokes(&[stroke], &bounds, 10, 10).unwrap();
        let lines: Vec<&str> = result.lines().collect();

        // Diagonal should have at least one filled cell per row
        for line in &lines {
            assert!(
                line.contains('█'),
                "each row should have at least one filled cell on diagonal"
            );
        }
    }

    #[test]
    fn rasterize_single_dot() {
        let bounds = CanvasBounds {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let stroke = vec![[50.0, 50.0]];
        let result = rasterize_strokes(&[stroke], &bounds, 10, 10).unwrap();

        // Should have exactly one filled cell
        let filled_count = result.chars().filter(|&c| c == '█').count();
        assert_eq!(
            filled_count, 1,
            "single dot should produce exactly one filled cell"
        );
    }

    #[test]
    fn rasterize_empty_strokes() {
        let bounds = CanvasBounds {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let result = rasterize_strokes(&[], &bounds, 10, 10);
        assert!(result.is_none(), "empty strokes should return None");
    }

    #[test]
    fn rasterize_zero_area_bounds() {
        let bounds = CanvasBounds {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 100.0,
        };
        let stroke = vec![[0.0, 0.0], [0.0, 100.0]];
        let result = rasterize_strokes(&[stroke], &bounds, 10, 10);
        assert!(result.is_none(), "zero-width bounds should return None");
    }

    #[test]
    fn rasterize_multiple_strokes() {
        let bounds = CanvasBounds {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        // Two horizontal lines: one at top, one at bottom
        let stroke1 = vec![[0.0, 0.0], [100.0, 0.0]];
        let stroke2 = vec![[0.0, 100.0], [100.0, 100.0]];
        let result = rasterize_strokes(&[stroke1, stroke2], &bounds, 10, 10).unwrap();
        let lines: Vec<&str> = result.lines().collect();

        // First row should be filled
        assert!(
            lines[0].chars().all(|c| c == '█'),
            "top row should be filled"
        );
        // Last row should be filled
        assert!(
            lines.last().unwrap().chars().all(|c| c == '█'),
            "bottom row should be filled"
        );
    }

    #[test]
    fn rasterize_trims_trailing_empty_rows() {
        // Stroke only in the top quarter — bottom rows should be trimmed
        let bounds = CanvasBounds {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let stroke = vec![[0.0, 0.0], [100.0, 0.0]];
        let result = rasterize_strokes(&[stroke], &bounds, 10, 20).unwrap();
        let lines: Vec<&str> = result.lines().collect();

        // Should have trimmed — far fewer than 20 rows
        assert!(lines.len() < 20, "trailing empty rows should be trimmed");
        assert!(lines[0].contains('█'), "first row should have content");
    }

    // ========================================================================
    // Stroke classification — Integration tests
    // ========================================================================

    #[test]
    fn node_with_freedraw_inside() {
        // Node at (0, 0, 200, 100), freedraw inside at base (50, 30) with relative points
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "My Node");
        elements.push(make_freedraw(
            "fd1",
            50.0,
            30.0,
            vec![vec![0.0, 0.0], vec![20.0, 10.0], vec![40.0, 0.0]],
        ));

        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 1);
        assert!(
            snapshot.nodes[0].sketch.is_some(),
            "freedraw inside node should produce a sketch"
        );
    }

    #[test]
    fn freedraw_outside_node() {
        // Node at (0, 0, 200, 100), freedraw far away at (1000, 1000)
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "My Node");
        elements.push(make_freedraw(
            "fd1",
            1000.0,
            1000.0,
            vec![vec![0.0, 0.0], vec![20.0, 10.0]],
        ));

        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 1);
        assert!(
            snapshot.nodes[0].sketch.is_none(),
            "freedraw outside node should not produce a sketch"
        );
    }

    #[test]
    fn node_with_line_inside() {
        // Node at (0, 0, 200, 100), line inside
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "My Node");
        elements.push(make_line(
            "ln1",
            10.0,
            10.0,
            vec![vec![0.0, 0.0], vec![100.0, 50.0]],
        ));

        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 1);
        assert!(
            snapshot.nodes[0].sketch.is_some(),
            "line inside node should produce a sketch"
        );
    }

    #[test]
    fn freedraw_spanning_two_nodes() {
        // Two nodes side by side, freedraw spans both — should be discarded
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Node A");
        elements.extend(make_rect("r2", 200.0, 0.0, 200.0, 100.0, "Node B"));
        // Freedraw at (100, 50) with points spanning from -50 to +150 — overlaps both nodes
        elements.push(make_freedraw(
            "fd1",
            100.0,
            50.0,
            vec![
                vec![-50.0, 0.0], // absolute: (50, 50) — inside r1
                vec![150.0, 0.0], // absolute: (250, 50) — inside r2
            ],
        ));

        let snapshot = classify_board(&elements);

        assert_eq!(snapshot.nodes.len(), 2);
        let node_a = snapshot
            .nodes
            .iter()
            .find(|n| n.element_id == "r1")
            .unwrap();
        let node_b = snapshot
            .nodes
            .iter()
            .find(|n| n.element_id == "r2")
            .unwrap();
        assert!(
            node_a.sketch.is_none(),
            "stroke spanning two nodes should be discarded"
        );
        assert!(
            node_b.sketch.is_none(),
            "stroke spanning two nodes should be discarded"
        );
    }

    #[test]
    fn deleted_freedraw_skipped() {
        let mut elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "My Node");
        elements.push(ExcalidrawElement::Freedraw(FreedrawElement {
            id: "fd1".to_string(),
            x: 50.0,
            y: 50.0,
            is_deleted: true,
            points: vec![vec![0.0, 0.0], vec![20.0, 10.0]],
        }));

        let snapshot = classify_board(&elements);

        assert!(
            snapshot.nodes[0].sketch.is_none(),
            "deleted freedraw should be skipped"
        );
    }

    #[test]
    fn node_without_strokes_has_no_sketch() {
        let elements = make_rect("r1", 0.0, 0.0, 200.0, 100.0, "Plain Node");
        let snapshot = classify_board(&elements);

        assert!(
            snapshot.nodes[0].sketch.is_none(),
            "node without strokes should have no sketch"
        );
    }

    #[test]
    fn diff_ignores_sketch_changes() {
        // Two identical nodes except sketch differs — should NOT be reported as updated
        let mut prev_node = make_canvas_node("n1", "Same text", 0.0, 0.0);
        prev_node.sketch = None;

        let mut curr_node = make_canvas_node("n1", "Same text", 0.0, 0.0);
        curr_node.sketch = Some("██··██".to_string());

        let previous = make_snapshot(vec![prev_node], vec![], vec![]);
        let current = make_snapshot(vec![curr_node], vec![], vec![]);

        let changeset = diff_snapshots(&previous, &current);

        assert!(
            changeset.updated_nodes.is_empty(),
            "sketch change alone should not trigger update"
        );
        assert!(changeset.new_nodes.is_empty());
        assert!(changeset.deleted_node_ids.is_empty());
        assert!(changeset.moved_nodes.is_empty());
    }

    // ========================================================================
    // Changeset Filter — Helpers
    // ========================================================================

    use crate::server::hub::board_serializer::filter_changeset;

    fn make_node_update(id: &str, old_text: &str, new_text: &str) -> NodeUpdate {
        NodeUpdate {
            element_id: id.to_string(),
            old_text: old_text.to_string(),
            new_text: new_text.to_string(),
            old_annotations: vec![],
            new_annotations: vec![],
        }
    }

    fn make_node_update_with_annotations(
        id: &str,
        old_text: &str,
        new_text: &str,
        old_ann: Vec<&str>,
        new_ann: Vec<&str>,
    ) -> NodeUpdate {
        NodeUpdate {
            element_id: id.to_string(),
            old_text: old_text.to_string(),
            new_text: new_text.to_string(),
            old_annotations: old_ann.into_iter().map(String::from).collect(),
            new_annotations: new_ann.into_iter().map(String::from).collect(),
        }
    }

    fn make_node_move(id: &str, old_x: f64, old_y: f64, new_x: f64, new_y: f64) -> NodeMove {
        NodeMove {
            element_id: id.to_string(),
            old_bounds: CanvasBounds {
                x: old_x,
                y: old_y,
                width: 200.0,
                height: 100.0,
            },
            new_bounds: CanvasBounds {
                x: new_x,
                y: new_y,
                width: 200.0,
                height: 100.0,
            },
        }
    }

    fn empty_changeset() -> CanvasChangeset {
        CanvasChangeset {
            new_nodes: vec![],
            updated_nodes: vec![],
            deleted_node_ids: vec![],
            moved_nodes: vec![],
            new_edges: vec![],
            deleted_edge_ids: vec![],
            rewired_edges: vec![],
        }
    }

    // ========================================================================
    // Changeset Filter — Pan detection
    // ========================================================================

    #[test]
    fn filter_pan_all_same_delta() {
        let mut changeset = empty_changeset();
        changeset.moved_nodes = vec![
            make_node_move("n1", 0.0, 0.0, 50.0, 30.0),
            make_node_move("n2", 200.0, 0.0, 250.0, 30.0),
            make_node_move("n3", 400.0, 0.0, 450.0, 30.0),
        ];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert!(result.agentless.moved_nodes.is_empty(), "all moves are pan");
        assert_eq!(result.noise.len(), 3);
        assert!(result
            .noise
            .iter()
            .all(|n| n.reason == NoiseReason::CanvasPan));
    }

    #[test]
    fn filter_pan_different_deltas() {
        let mut changeset = empty_changeset();
        changeset.moved_nodes = vec![
            make_node_move("n1", 0.0, 0.0, 50.0, 30.0),
            make_node_move("n2", 200.0, 0.0, 300.0, 0.0), // different delta
            make_node_move("n3", 400.0, 0.0, 450.0, 30.0),
        ];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(
            result.agentless.moved_nodes.len(),
            3,
            "different deltas survive"
        );
        assert!(result.noise.is_empty());
    }

    #[test]
    fn filter_pan_single_move_not_pan() {
        let mut changeset = empty_changeset();
        changeset.moved_nodes = vec![make_node_move("n1", 0.0, 0.0, 50.0, 30.0)];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(
            result.agentless.moved_nodes.len(),
            1,
            "single move always survives"
        );
        assert!(result.noise.is_empty());
    }

    #[test]
    fn filter_pan_empty_moves() {
        let changeset = empty_changeset();

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert!(result.agentless.moved_nodes.is_empty());
        assert!(result.noise.is_empty());
    }

    #[test]
    fn filter_pan_within_epsilon() {
        // Deltas differ by 0.5px — within default epsilon of 1.0
        let mut changeset = empty_changeset();
        changeset.moved_nodes = vec![
            make_node_move("n1", 0.0, 0.0, 50.0, 30.0),
            make_node_move("n2", 200.0, 0.0, 250.5, 29.5), // delta: (50.5, 29.5) vs (50, 30)
        ];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert!(
            result.agentless.moved_nodes.is_empty(),
            "within epsilon → pan"
        );
        assert_eq!(result.noise.len(), 2);
    }

    // ========================================================================
    // Changeset Filter — Whitespace normalization
    // ========================================================================

    #[test]
    fn filter_whitespace_only_text() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update("n1", "  hello   world ", "hello world")];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert!(result.meaningful.is_empty());
        assert_eq!(result.noise.len(), 1);
        assert_eq!(result.noise[0].reason, NoiseReason::WhitespaceOnly);
    }

    #[test]
    fn filter_whitespace_with_real_change() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes =
            vec![make_node_update("n1", "  hello   world ", "hello universe")];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1, "real change survives");
        assert!(result.noise.is_empty());
    }

    #[test]
    fn filter_whitespace_annotation_only() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update_with_annotations(
            "n1",
            "same text",
            "same text",
            vec!["  old  note  "],
            vec!["old note"],
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert!(result.meaningful.is_empty());
        assert_eq!(result.noise.len(), 1);
        assert_eq!(result.noise[0].reason, NoiseReason::WhitespaceOnly);
    }

    #[test]
    fn filter_whitespace_tabs_and_newlines() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update("n1", "hello\t\tworld\n\n", "hello world")];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert!(result.meaningful.is_empty());
        assert_eq!(result.noise[0].reason, NoiseReason::WhitespaceOnly);
    }

    // ========================================================================
    // Changeset Filter — Oscillation detection
    // ========================================================================

    #[test]
    fn filter_oscillation_matches_baseline() {
        let baseline = make_snapshot(
            vec![make_canvas_node("n1", "original text", 0.0, 0.0)],
            vec![],
            vec![],
        );

        let mut changeset = empty_changeset();
        // User changed "original text" → "edited text" → "original text"
        // diff sees: old="edited text", new="original text"
        changeset.updated_nodes = vec![make_node_update("n1", "edited text", "original text")];

        let result = filter_changeset(&changeset, &[], Some(&baseline), &FilterConfig::default());

        assert!(result.meaningful.is_empty());
        assert_eq!(result.noise.len(), 1);
        assert_eq!(result.noise[0].reason, NoiseReason::Oscillation);
    }

    #[test]
    fn filter_oscillation_no_baseline() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update("n1", "edited text", "original text")];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(
            result.meaningful.len(),
            1,
            "no baseline → oscillation skipped"
        );
    }

    #[test]
    fn filter_oscillation_no_match() {
        let baseline = make_snapshot(
            vec![make_canvas_node("n1", "original text", 0.0, 0.0)],
            vec![],
            vec![],
        );

        let mut changeset = empty_changeset();
        changeset.updated_nodes =
            vec![make_node_update("n1", "edited text", "completely new text")];

        let result = filter_changeset(&changeset, &[], Some(&baseline), &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1, "no match → survives");
    }

    #[test]
    fn filter_oscillation_node_not_in_baseline() {
        let baseline = make_snapshot(vec![], vec![], vec![]);

        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update("n1", "old text", "new text")];

        let result = filter_changeset(&changeset, &[], Some(&baseline), &FilterConfig::default());

        assert_eq!(
            result.meaningful.len(),
            1,
            "node not in baseline → survives"
        );
    }

    // ========================================================================
    // Changeset Filter — Reorder detection
    // ========================================================================

    #[test]
    fn filter_reorder_same_lines() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update(
            "n1",
            "Line A\nLine B\nLine C",
            "Line C\nLine A\nLine B",
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert!(result.meaningful.is_empty());
        assert_eq!(result.noise.len(), 1);
        assert_eq!(result.noise[0].reason, NoiseReason::ReorderOnly);
    }

    #[test]
    fn filter_reorder_different_content() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update("n1", "Line A\nLine B", "Line C\nLine A")];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1, "different content survives");
    }

    #[test]
    fn filter_reorder_different_count() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update(
            "n1",
            "Line A\nLine B\nLine C",
            "Line A\nLine B",
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1, "different line count survives");
    }

    #[test]
    fn filter_reorder_blank_lines_ignored() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update(
            "n1",
            "Line A\n\nLine B\nLine C",
            "Line C\nLine A\n\nLine B",
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert!(result.meaningful.is_empty());
        assert_eq!(result.noise[0].reason, NoiseReason::ReorderOnly);
    }

    // ========================================================================
    // Changeset Filter — Token scoring
    // ========================================================================

    #[test]
    fn score_low_significance() {
        // 1 word changed out of 50 words → 2 token diffs / 50 total = 0.04 < 0.05
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update(
            "n1",
            "the quick brown fox jumps over the lazy dog and then runs across the big open field to find the hidden treasure chest buried deep beneath the ancient oak tree standing tall in the middle of a vast green meadow near the old stone wall",
            "the quick brown fox jumps over the lazy dog and then runs across the big open field to find the hidden treasure chest buried deep beneath the ancient oak tree standing tall in the middle of a vast green meadow near the old stone fence",
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1);
        let sig = result.meaningful[0].significance();
        assert_eq!(sig, ChangeSignificance::Low, "<5% change → Low");
    }

    #[test]
    fn score_medium_significance() {
        // 1 word changed out of 15 words → 2 token diffs / 15 total ≈ 0.13
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update(
            "n1",
            "query the database for user records and return them as json formatted output today",
            "query the database for user records and return them as json formatted result today",
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1);
        let sig = result.meaningful[0].significance();
        assert_eq!(sig, ChangeSignificance::Medium, "~13% change → Medium");
    }

    #[test]
    fn score_high_significance() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update(
            "n1",
            "Research competitors",
            "Research competitors\n- Q3 and Q4 pricing data\n- Compare year-over-year trends\n- Flag anomalies above 10%",
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1);
        let sig = result.meaningful[0].significance();
        assert_eq!(sig, ChangeSignificance::High, "major rewrite → High");
    }

    #[test]
    fn score_new_node_always_high() {
        let mut changeset = empty_changeset();
        changeset.new_nodes = vec![make_canvas_node("n1", "Brand new node", 0.0, 0.0)];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1);
        assert_eq!(
            result.meaningful[0].significance(),
            ChangeSignificance::High
        );
    }

    #[test]
    fn score_new_edge_always_medium() {
        let mut changeset = empty_changeset();
        changeset.new_edges = vec![make_canvas_edge("e1", "n1", "n2")];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1);
        assert_eq!(
            result.meaningful[0].significance(),
            ChangeSignificance::Medium
        );
    }

    // ========================================================================
    // Changeset Filter — Token scoring (blind-spot coverage)
    // ========================================================================

    #[test]
    fn score_morphological_variant() {
        // "database" → "databases" and "stores" → "store" — similar words, not a rewrite
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update(
            "n1",
            "the database stores user records and returns them as json formatted output for the client application today",
            "the databases store user records and returns them as json formatted output for the client application today",
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1);
        let sig = result.meaningful[0].significance();
        assert_eq!(
            sig,
            ChangeSignificance::Low,
            "morphological variants should score Low"
        );
    }

    #[test]
    fn score_hyphenation_change() {
        // "JSON formatted" → "JSON-formatted" — formatting change, not a rewrite
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update(
            "n1",
            "return the data as JSON formatted output for the downstream service to consume",
            "return the data as JSON-formatted output for the downstream service to consume",
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1);
        let sig = result.meaningful[0].significance();
        assert!(
            sig == ChangeSignificance::Low || sig == ChangeSignificance::Medium,
            "hyphenation change should score Low or Medium, got {:?}",
            sig
        );
    }

    #[test]
    fn score_completely_different_words() {
        // Single word with zero character overlap — should score High
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update("n1", "database", "frontend")];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1);
        let sig = result.meaningful[0].significance();
        assert_eq!(
            sig,
            ChangeSignificance::High,
            "completely different words should score High"
        );
    }

    #[test]
    fn score_similar_prefix_words() {
        // "userId" → "userIds" in a longer text — similar prefix, should be Low
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update(
            "n1",
            "fetch the userId from the authentication service and validate the token before returning the response payload",
            "fetch the userIds from the authentication service and validate the token before returning the response payload",
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.meaningful.len(), 1);
        let sig = result.meaningful[0].significance();
        assert_eq!(
            sig,
            ChangeSignificance::Low,
            "similar prefix words should score Low in long text"
        );
    }

    // ========================================================================
    // Changeset Filter — Topological sort
    // ========================================================================

    #[test]
    fn filter_topo_sort_upstream_first() {
        // Chain: A → B → C — all updated
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![
            make_node_update("C", "old C", "new C"),
            make_node_update("A", "old A", "new A"),
            make_node_update("B", "old B", "new B"),
        ];

        let edges = vec![
            make_canvas_edge("e1", "A", "B"),
            make_canvas_edge("e2", "B", "C"),
        ];

        let result = filter_changeset(&changeset, &edges, None, &FilterConfig::default());

        let ids: Vec<&str> = result.meaningful.iter().map(|c| c.element_id()).collect();
        assert_eq!(ids, vec!["A", "B", "C"], "should be in topological order");
    }

    #[test]
    fn filter_topo_sort_no_edges() {
        let mut changeset = empty_changeset();
        changeset.new_nodes = vec![
            make_canvas_node("n2", "Second", 200.0, 0.0),
            make_canvas_node("n1", "First", 0.0, 0.0),
        ];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        // No edges — stable insertion order preserved
        let ids: Vec<&str> = result.meaningful.iter().map(|c| c.element_id()).collect();
        assert_eq!(ids, vec!["n2", "n1"], "no edges → insertion order");
    }

    #[test]
    fn filter_topo_sort_edges_after_endpoints() {
        let mut changeset = empty_changeset();
        changeset.new_nodes = vec![
            make_canvas_node("B", "Node B", 200.0, 0.0),
            make_canvas_node("A", "Node A", 0.0, 0.0),
        ];
        changeset.new_edges = vec![make_canvas_edge("e1", "A", "B")];

        let edges = vec![make_canvas_edge("e1", "A", "B")];

        let result = filter_changeset(&changeset, &edges, None, &FilterConfig::default());

        let ids: Vec<&str> = result.meaningful.iter().map(|c| c.element_id()).collect();
        // A first (upstream), B second, then edge e1 (after both endpoints)
        assert_eq!(ids, vec!["A", "B", "e1"]);
    }

    #[test]
    fn filter_topo_sort_mixed_types() {
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update("B", "old B", "new B")];
        changeset.new_nodes = vec![make_canvas_node("A", "Node A", 0.0, 0.0)];
        changeset.new_edges = vec![make_canvas_edge("e1", "A", "B")];

        let edges = vec![make_canvas_edge("e1", "A", "B")];

        let result = filter_changeset(&changeset, &edges, None, &FilterConfig::default());

        let ids: Vec<&str> = result.meaningful.iter().map(|c| c.element_id()).collect();
        assert_eq!(ids[0], "A", "upstream node first");
        assert_eq!(ids[1], "B", "downstream node second");
        assert_eq!(ids[2], "e1", "edge after both endpoints");
    }

    // ========================================================================
    // Changeset Filter — Aggregate score & dispatch decision
    // ========================================================================

    #[test]
    fn filter_aggregate_low_score_still_dispatches() {
        // One Low-significance update. Score = 0.2, default threshold = 0.1
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update(
            "n1",
            "the quick brown fox jumps over the lazy dog and then runs across the big open field to find the hidden treasure chest buried deep beneath the ancient oak tree standing tall in the middle of a vast green meadow near the old stone wall",
            "the quick brown fox jumps over the lazy dog and then runs across the big open field to find the hidden treasure chest buried deep beneath the ancient oak tree standing tall in the middle of a vast green meadow near the old stone fence",
        )];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert!(result.should_dispatch, "Low score (0.2) >= threshold (0.1)");
    }

    #[test]
    fn filter_aggregate_empty_meaningful() {
        // All changes filtered as noise
        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![make_node_update("n1", "  hello  ", "hello")];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.aggregate_score, 0.0);
        assert!(
            !result.should_dispatch,
            "no meaningful changes → don't dispatch"
        );
    }

    #[test]
    fn filter_aggregate_custom_threshold() {
        let config = FilterConfig {
            dispatch_threshold: 0.8,
            pan_epsilon: 1.0,
        };

        let mut changeset = empty_changeset();
        changeset.new_edges = vec![make_canvas_edge("e1", "n1", "n2")];

        let result = filter_changeset(&changeset, &[], None, &config);

        // One Medium edge → score = 0.5 < 0.8
        assert!(!result.should_dispatch, "score 0.5 < threshold 0.8");
    }

    // ========================================================================
    // Changeset Filter — Integration
    // ========================================================================

    #[test]
    fn filter_full_pipeline_mixed() {
        let mut changeset = empty_changeset();

        // 3 pan moves (same delta)
        changeset.moved_nodes = vec![
            make_node_move("m1", 0.0, 0.0, 50.0, 30.0),
            make_node_move("m2", 200.0, 0.0, 250.0, 30.0),
            make_node_move("m3", 400.0, 0.0, 450.0, 30.0),
        ];

        // 1 whitespace-only update
        changeset
            .updated_nodes
            .push(make_node_update("ws1", "  hello  world  ", "hello world"));

        // 1 real text change
        changeset.updated_nodes.push(make_node_update(
            "real1",
            "Research competitors",
            "Research competitors\n- Q3 pricing\n- Q4 pricing\n- Year-over-year trends",
        ));

        // 1 new node
        changeset.new_nodes = vec![make_canvas_node("new1", "Brand new node", 600.0, 0.0)];

        // 1 deleted node
        changeset.deleted_node_ids = vec!["del1".to_string()];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        // 3 pan noise + 1 whitespace noise = 4 noise entries
        assert_eq!(result.noise.len(), 4);

        // 2 meaningful: 1 real update + 1 new node
        assert_eq!(result.meaningful.len(), 2);

        // 1 agentless deletion
        assert_eq!(result.agentless.deleted_node_ids, vec!["del1"]);

        // Pan moves filtered out
        assert!(result.agentless.moved_nodes.is_empty());

        assert!(result.should_dispatch);
    }

    #[test]
    fn filter_full_pipeline_all_noise() {
        let mut changeset = empty_changeset();

        // Only whitespace changes and pan moves
        changeset.updated_nodes = vec![
            make_node_update("n1", "  hello  ", "hello"),
            make_node_update("n2", "world  ", "world"),
        ];
        changeset.moved_nodes = vec![
            make_node_move("m1", 0.0, 0.0, 50.0, 30.0),
            make_node_move("m2", 200.0, 0.0, 250.0, 30.0),
        ];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert!(result.meaningful.is_empty());
        assert!(!result.should_dispatch, "all noise → no dispatch");
        assert_eq!(result.noise.len(), 4); // 2 whitespace + 2 pan
    }

    #[test]
    fn filter_full_pipeline_with_baseline() {
        let baseline = make_snapshot(
            vec![
                make_canvas_node("n1", "original text", 0.0, 0.0),
                make_canvas_node("n2", "other original", 200.0, 0.0),
            ],
            vec![],
            vec![],
        );

        let mut changeset = empty_changeset();
        changeset.updated_nodes = vec![
            // n1: oscillation (back to baseline)
            make_node_update("n1", "edited text", "original text"),
            // n2: real change (not back to baseline)
            make_node_update(
                "n2",
                "other original",
                "completely rewritten content for node two",
            ),
        ];

        let result = filter_changeset(&changeset, &[], Some(&baseline), &FilterConfig::default());

        assert_eq!(result.noise.len(), 1);
        assert_eq!(result.noise[0].element_id, "n1");
        assert_eq!(result.noise[0].reason, NoiseReason::Oscillation);

        assert_eq!(result.meaningful.len(), 1);
        assert_eq!(result.meaningful[0].element_id(), "n2");
    }

    // ========================================================================
    // Changeset Filter — Tier 1 pass-through
    // ========================================================================

    #[test]
    fn filter_agentless_pass_through() {
        let mut changeset = empty_changeset();
        changeset.deleted_node_ids = vec!["d1".to_string(), "d2".to_string()];
        changeset.deleted_edge_ids = vec!["de1".to_string()];
        changeset.rewired_edges = vec![EdgeRewire {
            element_id: "re1".to_string(),
            old_source: "a".to_string(),
            old_target: "b".to_string(),
            new_source: "a".to_string(),
            new_target: "c".to_string(),
        }];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        assert_eq!(result.agentless.deleted_node_ids, vec!["d1", "d2"]);
        assert_eq!(result.agentless.deleted_edge_ids, vec!["de1"]);
        assert_eq!(result.agentless.rewired_edges.len(), 1);
        assert_eq!(result.agentless.rewired_edges[0].new_target, "c");
    }

    #[test]
    fn filter_agentless_moved_nodes_after_pan() {
        let mut changeset = empty_changeset();
        changeset.moved_nodes = vec![
            // These two share same delta → pan
            make_node_move("pan1", 0.0, 0.0, 50.0, 30.0),
            make_node_move("pan2", 200.0, 0.0, 250.0, 30.0),
        ];

        let result = filter_changeset(&changeset, &[], None, &FilterConfig::default());

        // Pan moves filtered out of agentless
        assert!(result.agentless.moved_nodes.is_empty());
        assert_eq!(result.noise.len(), 2);
        assert!(result
            .noise
            .iter()
            .all(|n| n.reason == NoiseReason::CanvasPan));
    }
}
