#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::markup::{xml_escape, XmlBuilder};
    use crate::server::hub::board_state::render;
    use crate::server::hub::board_state::types::*;

    // ========================================================================
    // Test helpers
    // ========================================================================

    fn make_agent(name: &str, caps: &[&str], receives: &[&str], desc: &str) -> AgentSnapshot {
        AgentSnapshot {
            id: Uuid::new_v4(),
            name: name.to_string(),
            role_description: desc.to_string(),
            capabilities: caps.iter().map(|s| s.to_string()).collect(),
            receives_from: receives.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn make_node(name: &str, agents: Vec<AgentSnapshot>) -> NodeSnapshot {
        let agent_count = agents.len();
        NodeSnapshot {
            id: Uuid::new_v4(),
            ref_id: None,
            name: name.to_string(),
            protocol: "workforce".to_string(),
            status: if agent_count > 0 {
                "configured".to_string()
            } else {
                "idle".to_string()
            },
            task: "Research best practices".to_string(),
            capabilities: vec!["web_search".to_string()],
            failure_mode: "fail_fast".to_string(),
            summary: format!(
                "{} agent{}",
                agent_count,
                if agent_count == 1 { "" } else { "s" }
            ),
            agents,
            input_ports: vec![],
            output_ports: vec![],
            incoming_context: vec![],
            notes: String::new(),
            asking: None,
            receives: None,
        }
    }

    fn make_board(nodes: Vec<NodeSnapshot>) -> BoardSnapshot {
        BoardSnapshot {
            workflow_name: "Test Workflow".to_string(),
            workflow_id: Uuid::new_v4(),
            nodes,
            available_capabilities: vec!["github".to_string(), "web_search".to_string()],
        }
    }

    fn make_own_node_snapshot(node: NodeSnapshot) -> BoardSnapshot {
        BoardSnapshot {
            workflow_name: String::new(),
            workflow_id: Uuid::new_v4(),
            nodes: vec![node],
            available_capabilities: vec![],
        }
    }

    // ========================================================================
    // L1 — Manager Assistant
    // ========================================================================

    #[test]
    fn render_l1_multi_node_with_asking() {
        let mut node1 = make_node(
            "Collector",
            vec![
                make_agent("Scraper", &["web_search"], &[], "Scrapes web pages"),
                make_agent("Formatter", &[], &["Scraper"], "Formats output"),
            ],
        );
        node1.asking = Some("Which companies should we target?".to_string());

        let node2 = make_node("Analyzer", vec![]);

        let snapshot = make_board(vec![node1, node2]);
        let xml = render::render(&snapshot, BoardStateVariant::ManagerAssistant);

        assert!(xml.starts_with("<board_state>\n"));
        assert!(xml.ends_with("</board_state>\n"));
        assert!(xml.contains("<workflow name=\"Test Workflow\""));
        assert!(xml.contains("status=\"configuring\""));
        assert!(xml.contains("<node name=\"Collector\""));
        assert!(xml.contains("agents=\"Scraper, Formatter\""));
        assert!(xml.contains("<asking>Which companies should we target?</asking>"));
        assert!(xml.contains("<node name=\"Analyzer\""));
        assert!(xml.contains("status=\"idle\""));
        // L1 should NOT include ids
        assert!(!xml.contains(" id=\""));
    }

    #[test]
    fn render_l1_no_asking() {
        let node = make_node("Simple", vec![make_agent("Worker", &[], &[], "Does work")]);
        let snapshot = make_board(vec![node]);
        let xml = render::render(&snapshot, BoardStateVariant::ManagerAssistant);

        assert!(!xml.contains("<asking>"));
    }

    // ========================================================================
    // L2 — Manager Builder
    // ========================================================================

    #[test]
    fn render_l2_with_ids_and_capabilities() {
        let mut node = make_node(
            "Collector",
            vec![make_agent("Scraper", &["web_search"], &[], "Scrapes data")],
        );
        node.ref_id = Some("workforce-1".to_string());
        let snapshot = make_board(vec![node]);
        let xml = render::render(&snapshot, BoardStateVariant::ManagerBuilder);

        assert!(xml.contains("<workflow name=\"Test Workflow\""));
        // L2 includes workflow id
        assert!(xml.contains(" id=\""));
        // L2 includes ref attribute before name
        assert!(xml.contains("ref=\"workforce-1\""));
        // L2 includes node id
        assert!(xml.contains("name=\"Collector\" id=\""));
        // L2 includes available_capabilities
        assert!(xml.contains("<available_capabilities>github, web_search</available_capabilities>"));
        // L2 should NOT include <asking>
        assert!(!xml.contains("<asking>"));
    }

    // ========================================================================
    // L3 — Node Assistant
    // ========================================================================

    #[test]
    fn render_l3_with_agents_and_incoming() {
        let mut node = make_node(
            "Research Team",
            vec![
                make_agent("Researcher", &["web_search"], &[], "Investigates sources"),
                make_agent("Synthesizer", &[], &["Researcher"], "Combines findings"),
            ],
        );
        node.incoming_context = vec![IncomingContextSnapshot {
            name: "Requirements".to_string(),
            source_mode: "context".to_string(),
            status: "populated".to_string(),
            preview: Some("Build a research pipeline for...".to_string()),
            word_count: Some(42),
        }];
        node.receives = Some("Context Node".to_string());

        let snapshot = make_own_node_snapshot(node);
        let xml = render::render(&snapshot, BoardStateVariant::NodeAssistant);

        assert!(xml.starts_with("<board_state>\n"));
        // L3 should NOT have <workflow> wrapper
        assert!(!xml.contains("<workflow"));
        // Node attributes
        assert!(xml.contains("task=\"Research best practices\""));
        assert!(xml.contains("capabilities=\"web_search\""));
        assert!(xml.contains("receives=\"Context Node\""));
        // Agent children with descriptions
        assert!(xml.contains("<agent name=\"Researcher\""));
        assert!(xml.contains("Investigates sources"));
        assert!(xml.contains("<agent name=\"Synthesizer\""));
        assert!(xml.contains("receives_from=\"Researcher\""));
        // L3 should NOT include agent ids
        assert!(!xml.contains("<agent name=\"Researcher\" id=\""));
        // Incoming ports
        assert!(xml.contains("<incoming>"));
        assert!(xml.contains("<port name=\"Requirements\""));
        assert!(xml.contains("status=\"populated\""));
        assert!(xml.contains("Build a research pipeline for..."));
    }

    #[test]
    fn render_l3_empty_node() {
        let node = make_node("Empty Node", vec![]);
        let snapshot = make_own_node_snapshot(node);
        let xml = render::render(&snapshot, BoardStateVariant::NodeAssistant);

        assert!(xml.contains("<node name=\"Empty Node\""));
        assert!(xml.contains("status=\"idle\""));
        assert!(!xml.contains("<agent"));
        assert!(!xml.contains("<incoming>"));
    }

    // ========================================================================
    // L4 — Dispatch
    // ========================================================================

    #[test]
    fn render_l4_full_detail() {
        let mut node = make_node(
            "Research Team",
            vec![
                make_agent("Researcher", &["web_search"], &[], "Investigates sources"),
                make_agent("Synthesizer", &[], &["Researcher"], "Combines findings"),
            ],
        );
        node.input_ports = vec![InputPortSnapshot {
            port_name: "requirements".to_string(),
            from_node: "Context Node".to_string(),
            schema: Some(r#"{"type": "string"}"#.to_string()),
            json_path: Some("$.output".to_string()),
        }];
        node.output_ports = vec![OutputPortSnapshot {
            port_name: "report".to_string(),
            to_node: "Writer Node".to_string(),
            schema: Some(r#"{"type": "string"}"#.to_string()),
        }];
        node.notes = "Focus on academic sources.".to_string();

        let snapshot = make_own_node_snapshot(node);
        let xml = render::render(&snapshot, BoardStateVariant::Dispatch);

        // No workflow wrapper
        assert!(!xml.contains("<workflow"));
        // Node has task attr
        assert!(xml.contains("task=\"Research best practices\""));
        // Input ports with schema
        assert!(xml.contains("<input_ports>"));
        assert!(xml.contains("<port name=\"requirements\" from=\"Context Node\">"));
        assert!(xml.contains(r#"<schema>{"type": "string"}</schema>"#));
        assert!(xml.contains("<json_path>$.output</json_path>"));
        // Output ports
        assert!(xml.contains("<output_ports>"));
        assert!(xml.contains("<port name=\"report\" to=\"Writer Node\">"));
        // Capabilities as child element
        assert!(xml.contains("<capabilities>web_search</capabilities>"));
        // Agent roster with ids
        assert!(xml.contains("<agent_roster>"));
        assert!(xml.contains("<agent name=\"Researcher\" id=\""));
        assert!(xml.contains("<role>Investigates sources</role>"));
        assert!(xml.contains("<depends_on>Researcher</depends_on>"));
        // Notes
        assert!(xml.contains("<notes>Focus on academic sources.</notes>"));
    }

    #[test]
    fn render_l4_no_ports_no_notes() {
        let node = make_node(
            "Minimal",
            vec![make_agent("Worker", &[], &[], "Does things")],
        );
        let snapshot = make_own_node_snapshot(node);
        let xml = render::render(&snapshot, BoardStateVariant::Dispatch);

        assert!(!xml.contains("<input_ports>"));
        assert!(!xml.contains("<output_ports>"));
        assert!(!xml.contains("<notes>"));
        assert!(xml.contains("<agent_roster>"));
    }

    // ========================================================================
    // Variant methods
    // ========================================================================

    #[test]
    fn variant_scope() {
        assert_eq!(BoardStateVariant::ManagerAssistant.scope(), Scope::AllNodes);
        assert_eq!(BoardStateVariant::ManagerBuilder.scope(), Scope::AllNodes);
        assert_eq!(BoardStateVariant::NodeAssistant.scope(), Scope::OwnNode);
        assert_eq!(BoardStateVariant::Dispatch.scope(), Scope::OwnNode);
    }

    #[test]
    fn variant_include_flags() {
        // L1
        assert!(BoardStateVariant::ManagerAssistant.include_asking());
        assert!(!BoardStateVariant::ManagerAssistant.include_node_ids());
        assert!(!BoardStateVariant::ManagerAssistant.include_capabilities());

        // L2
        assert!(!BoardStateVariant::ManagerBuilder.include_asking());
        assert!(BoardStateVariant::ManagerBuilder.include_node_ids());
        assert!(BoardStateVariant::ManagerBuilder.include_capabilities());

        // L3
        assert!(!BoardStateVariant::NodeAssistant.include_agent_ids());
        assert!(BoardStateVariant::NodeAssistant.include_agent_children());
        assert!(BoardStateVariant::NodeAssistant.include_ports());
        assert!(!BoardStateVariant::NodeAssistant.include_port_schemas());

        // L4
        assert!(BoardStateVariant::Dispatch.include_agent_ids());
        assert!(BoardStateVariant::Dispatch.include_port_schemas());
        assert!(BoardStateVariant::Dispatch.include_notes());
    }

    // ========================================================================
    // XML escaping
    // ========================================================================

    #[test]
    fn xml_escape_special_chars() {
        assert_eq!(xml_escape("a & b"), "a &amp; b");
        assert_eq!(xml_escape("<script>"), "&lt;script&gt;");
        assert_eq!(xml_escape(r#"say "hello""#), "say &quot;hello&quot;");
        assert_eq!(xml_escape("clean text"), "clean text");
    }

    // ========================================================================
    // XmlBuilder unit tests
    // ========================================================================

    #[test]
    fn builder_self_closing() {
        let xml = XmlBuilder::new("empty", 0).build();
        assert_eq!(xml, "<empty />\n");
    }

    #[test]
    fn builder_inline_text() {
        let xml = XmlBuilder::new("tag", 0).text("hello").build();
        assert_eq!(xml, "<tag>hello</tag>\n");
    }

    #[test]
    fn builder_attrs_and_children() {
        let xml = XmlBuilder::new("node", 0)
            .attr("name", "Test")
            .text("summary")
            .raw(&XmlBuilder::new("child", 1).text("inner").build())
            .build();

        assert!(xml.contains("<node name=\"Test\">"));
        assert!(xml.contains("  summary\n"));
        assert!(xml.contains("  <child>inner</child>"));
        assert!(xml.contains("</node>"));
    }

    #[test]
    fn builder_indent() {
        let xml = XmlBuilder::new("deep", 3).attr("x", "1").build();
        assert!(xml.starts_with("      <deep")); // 3 * 2 spaces
    }
}
