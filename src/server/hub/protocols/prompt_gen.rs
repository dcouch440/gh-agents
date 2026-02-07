//! Prompt injection auto-generation utilities for protocols.

use super::types::PortConfig;

/// Generate the decomp prompt injection: tells the orchestrator about available
/// agents/ports and the expected output format.
pub fn decomp_prompt(ports: &[PortConfig]) -> String {
    let mut lines = Vec::new();
    lines.push("## Task Decomposition Protocol".to_string());
    lines.push(String::new());
    lines.push(
        "You must decompose the given task and assign subtasks to the available agents below."
            .to_string(),
    );
    lines.push(
        "You may assign MULTIPLE tasks to the same port — each will be executed independently."
            .to_string(),
    );
    lines.push(
        "Respond with a JSON array where each item has a \"port\" and \"content\" field."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("### Available Agents".to_string());
    lines.push(String::new());

    for port in ports {
        let mut agent_line = format!(
            "- **Port: \"{}\"** ({}) — {}",
            port.port_name, port.agent_name, port.description
        );
        if !port.agent_tools.is_empty() {
            agent_line.push_str(&format!("\n  Tools: {}", port.agent_tools.join(", ")));
        }
        lines.push(agent_line);
    }

    lines.push(String::new());
    lines.push("### Output Format".to_string());
    lines.push(String::new());
    lines.push("```json".to_string());
    lines.push("[".to_string());
    if let Some(first) = ports.first() {
        lines.push(format!(
            "  {{\"port\": \"{}\", \"content\": {{...task details...}}}}",
            first.port_name
        ));
        if ports.len() > 1 {
            lines.push(format!(
                "  {{\"port\": \"{}\", \"content\": {{...task details...}}}}",
                ports[1].port_name
            ));
        }
    }
    lines.push("]".to_string());
    lines.push("```".to_string());

    lines.join("\n")
}

/// Generate the route prompt injection: tells the orchestrator to pick
/// exactly one agent to handle the task.
pub fn route_prompt(ports: &[PortConfig]) -> String {
    let mut lines = Vec::new();
    lines.push("## Routing Protocol".to_string());
    lines.push(String::new());
    lines.push(
        "Examine the input and route it to exactly ONE of the available agents below.".to_string(),
    );
    lines.push(
        "Respond with a JSON object containing a \"port\" and \"content\" field.".to_string(),
    );
    lines.push(String::new());
    lines.push("### Available Agents".to_string());
    lines.push(String::new());

    for port in ports {
        lines.push(format!(
            "- **Port: \"{}\"** ({}) — {}",
            port.port_name, port.agent_name, port.description
        ));
    }

    lines.push(String::new());
    lines.push("### Output Format".to_string());
    lines.push(String::new());
    lines.push("```json".to_string());
    lines.push("{\"port\": \"<chosen_port>\", \"content\": {...task details...}}".to_string());
    lines.push("```".to_string());

    lines.join("\n")
}

/// Generate the review prompt injection: tells the agent to review input
/// and provide a decision.
pub fn review_prompt(decisions: &[String]) -> String {
    let decision_list = decisions.join("\", \"");
    let mut lines = Vec::new();
    lines.push("## Review Protocol".to_string());
    lines.push(String::new());
    lines.push(
        "Review the provided input and make a decision. Provide detailed feedback.".to_string(),
    );
    lines.push(format!(
        "Your decision must be one of: \"{}\"",
        decision_list
    ));
    lines.push(String::new());
    lines.push("### Output Format".to_string());
    lines.push(String::new());
    lines.push("```json".to_string());
    lines.push(
        "{\"decision\": \"<your_decision>\", \"feedback\": \"<detailed_feedback>\"}".to_string(),
    );
    lines.push("```".to_string());

    lines.join("\n")
}

/// Generate the transform prompt injection: tells the agent to produce
/// structured output matching the provided schema.
pub fn transform_prompt(schema_description: Option<&str>) -> String {
    let mut lines = Vec::new();
    lines.push("## Transform Protocol".to_string());
    lines.push(String::new());
    lines.push(
        "Process the input and produce structured output matching the required schema.".to_string(),
    );

    if let Some(desc) = schema_description {
        lines.push(String::new());
        lines.push(format!("Schema description: {}", desc));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hub::protocols::types::PortConfig;
    use uuid::Uuid;

    fn make_ports() -> Vec<PortConfig> {
        vec![
            PortConfig {
                port_name: "frontend".to_string(),
                description: "Handles UI work".to_string(),
                agent_id: Uuid::new_v4(),
                agent_name: "FE Agent".to_string(),
                agent_tools: vec!["read_file".to_string(), "write_file".to_string()],
                display_order: 0,
            },
            PortConfig {
                port_name: "backend".to_string(),
                description: "Handles API work".to_string(),
                agent_id: Uuid::new_v4(),
                agent_name: "BE Agent".to_string(),
                agent_tools: vec!["run_tests".to_string()],
                display_order: 1,
            },
        ]
    }

    #[test]
    fn decomp_prompt_includes_all_ports() {
        let ports = make_ports();
        let prompt = decomp_prompt(&ports);
        assert!(prompt.contains("Port: \"frontend\""));
        assert!(prompt.contains("Port: \"backend\""));
        assert!(prompt.contains("FE Agent"));
        assert!(prompt.contains("BE Agent"));
        assert!(prompt.contains("Task Decomposition Protocol"));
    }

    #[test]
    fn decomp_prompt_includes_multi_assignment_language() {
        let ports = make_ports();
        let prompt = decomp_prompt(&ports);
        assert!(prompt.contains("MULTIPLE tasks to the same port"));
        assert!(prompt.contains("each will be executed independently"));
    }

    #[test]
    fn decomp_prompt_includes_agent_tools() {
        let ports = make_ports();
        let prompt = decomp_prompt(&ports);
        assert!(prompt.contains("Tools: read_file, write_file"));
        assert!(prompt.contains("Tools: run_tests"));
    }

    #[test]
    fn decomp_prompt_omits_tools_when_empty() {
        let ports = vec![PortConfig {
            port_name: "worker".to_string(),
            description: "General worker".to_string(),
            agent_id: Uuid::new_v4(),
            agent_name: "Worker".to_string(),
            agent_tools: vec![],
            display_order: 0,
        }];
        let prompt = decomp_prompt(&ports);
        assert!(!prompt.contains("Tools:"));
    }

    #[test]
    fn route_prompt_includes_all_ports() {
        let ports = make_ports();
        let prompt = route_prompt(&ports);
        assert!(prompt.contains("Port: \"frontend\""));
        assert!(prompt.contains("exactly ONE"));
    }

    #[test]
    fn review_prompt_includes_decisions() {
        let decisions = vec!["approve".to_string(), "reject".to_string()];
        let prompt = review_prompt(&decisions);
        assert!(prompt.contains("approve"));
        assert!(prompt.contains("reject"));
    }
}
