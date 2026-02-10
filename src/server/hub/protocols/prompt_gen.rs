//! Prompt injection auto-generation utilities for protocols.
//!
//! These prompts are appended to `step.prompt_template` and end up inside
//! `<task>` tags at runtime. The output schema is enforced separately via
//! `<schema>` in the system prompt, so these provide semantic guidance rather
//! than hard format constraints.

use super::types::PortConfig;

/// Generate the decomp prompt injection: instructs the orchestrator to analyze
/// the task, break it into subtasks, and assign each to a specialist agent.
pub fn decomp_prompt(ports: &[PortConfig]) -> String {
    let mut lines = Vec::new();

    lines.push(
        "Analyze the task above and break it down into subtasks \
         for the specialist agents below."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("Guidelines:".to_string());
    lines.push(
        "- Decompose into the fewest subtasks needed. \
         Each should be a self-contained unit of work."
            .to_string(),
    );
    lines.push(
        "- Provide each agent with enough context to execute independently \
         — they cannot see each other's work."
            .to_string(),
    );
    lines.push(
        "- You may assign multiple subtasks to the same agent \
         when the work covers distinct concerns within their expertise."
            .to_string(),
    );
    lines.push(
        "- If subtasks have ordering dependencies, \
         note them in the content field."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("Available Agents:".to_string());

    for port in ports {
        lines.push(String::new());
        lines.push(format!("Port \"{}\" — {}", port.port_name, port.agent_name));
        lines.push(format!("  {}", port.description));
        if !port.agent_tools.is_empty() {
            lines.push(format!("  Tools: {}", port.agent_tools.join(", ")));
        }
    }

    lines.push(String::new());
    lines.push(
        "Respond with a JSON array. Each element has a \"port\" field \
         matching an agent identifier above, and a \"content\" field \
         with the task details for that agent."
            .to_string(),
    );

    lines.join("\n")
}

/// Generate the route prompt injection: instructs the orchestrator to analyze
/// the input and route it to exactly one specialist agent.
pub fn route_prompt(ports: &[PortConfig]) -> String {
    let mut lines = Vec::new();

    lines.push(
        "Analyze the input above to determine which specialist agent \
         is the best fit, then route it to exactly one agent."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("Routing criteria:".to_string());
    lines.push("- Identify the core intent and requirements of the input.".to_string());
    lines.push(
        "- Match those requirements against each agent's \
         expertise and available tools."
            .to_string(),
    );
    lines.push(
        "- Select the single best match. If multiple agents could handle \
         the input, choose the one whose expertise most directly \
         addresses the primary need."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("Available Agents:".to_string());

    for port in ports {
        lines.push(String::new());
        lines.push(format!("Port \"{}\" — {}", port.port_name, port.agent_name));
        lines.push(format!("  {}", port.description));
        if !port.agent_tools.is_empty() {
            lines.push(format!("  Tools: {}", port.agent_tools.join(", ")));
        }
    }

    lines.push(String::new());
    lines.push(
        "Respond with a JSON object containing a \"port\" field \
         matching an agent identifier above, and a \"content\" field \
         with the task details for that agent."
            .to_string(),
    );

    lines.join("\n")
}

/// Generate the review prompt injection: instructs the agent to evaluate input
/// against quality criteria and provide a structured decision with feedback.
pub fn review_prompt(decisions: &[String]) -> String {
    let decision_list = decisions.join("\", \"");
    let mut lines = Vec::new();

    lines.push("Evaluate the input above and provide your assessment.".to_string());
    lines.push(String::new());
    lines.push("Evaluation criteria:".to_string());
    lines.push(
        "- Correctness: Does the content accurately fulfill \
         the original requirements?"
            .to_string(),
    );
    lines.push(
        "- Completeness: Are all expected elements present, \
         or is anything missing?"
            .to_string(),
    );
    lines.push(
        "- Quality: Is the output well-structured, clear, \
         and free of obvious errors?"
            .to_string(),
    );
    lines.push(String::new());
    lines.push(
        "Form your own independent assessment. Do not assume the input \
         is correct — verify claims and check for gaps."
            .to_string(),
    );
    lines.push(String::new());
    lines.push(format!(
        "Your decision must be one of: \"{}\"",
        decision_list
    ));
    lines.push(String::new());
    lines.push("Respond with a JSON object containing:".to_string());
    lines.push("- \"decision\": one of the valid decisions listed above".to_string());
    lines.push(
        "- \"feedback\": a specific explanation citing what works, \
         what does not, and what to change. Reference concrete details \
         rather than giving generic praise or criticism."
            .to_string(),
    );

    lines.join("\n")
}

/// Generate the transform prompt injection: instructs the agent to transform
/// the input into a structured format matching the output schema.
pub fn transform_prompt(schema_description: Option<&str>) -> String {
    let mut lines = Vec::new();

    lines.push(
        "Transform the input above into the structured format \
         defined by the output schema."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("Steps:".to_string());
    lines.push("1. Identify the relevant data from the input.".to_string());
    lines.push(
        "2. Map each piece of data to the corresponding field \
         in the schema."
            .to_string(),
    );
    lines.push(
        "3. Ensure all required fields are populated \
         and values match the expected types."
            .to_string(),
    );
    lines.push(String::new());
    lines.push(
        "Your response is parsed directly by a JSON parser \
         — output only the JSON object."
            .to_string(),
    );

    if let Some(desc) = schema_description {
        lines.push(String::new());
        lines.push(format!("Schema context: {}", desc));
    }

    lines.join("\n")
}

/// Generate the documenter prompt injection: instructs the strategist to plan
/// research and writing for each requested document.
pub fn documenter_prompt(doc_defs: &[serde_json::Value], capabilities: &[String]) -> String {
    let mut lines = Vec::new();

    lines.push(
        "You are a Document Strategist. Your job is to plan how each \
         requested document should be researched and written."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("Requested Documents:".to_string());

    for (i, def) in doc_defs.iter().enumerate() {
        let name = def["name"].as_str().unwrap_or("Unnamed");
        let description = def["description"].as_str().unwrap_or("");
        let target_length = def["target_length"].as_i64().unwrap_or(2000);

        lines.push(String::new());
        if description.is_empty() {
            lines.push(format!(
                "{}. \"{}\" (target: ~{} characters)",
                i + 1,
                name,
                target_length
            ));
        } else {
            lines.push(format!(
                "{}. \"{}\" — {} (target: ~{} characters)",
                i + 1,
                name,
                description,
                target_length
            ));
        }
    }

    if !capabilities.is_empty() {
        lines.push(String::new());
        lines.push("Available Research Capabilities:".to_string());
        for cap in capabilities {
            lines.push(format!("- {}", cap));
        }
    }

    lines.push(String::new());
    lines.push("For each document, provide:".to_string());
    lines.push(
        "- document_name: must match one of the document names listed above exactly".to_string(),
    );
    lines.push(
        "- research_strategy: a step-by-step plan for gathering the information \
         needed to write this document"
            .to_string(),
    );
    lines.push(
        "- required_capabilities: which capabilities the researcher needs \
         from the list above (empty array if no research tools are needed)"
            .to_string(),
    );
    lines.push(
        "- writer_prompt: detailed instructions for the writer, including \
         tone, structure, target audience, and focus areas"
            .to_string(),
    );
    lines.push(String::new());
    lines.push(
        "Respond with a JSON object containing a \"document_plans\" array \
         with one entry per document."
            .to_string(),
    );

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
                content_schema: None,
            },
            PortConfig {
                port_name: "backend".to_string(),
                description: "Handles API work".to_string(),
                agent_id: Uuid::new_v4(),
                agent_name: "BE Agent".to_string(),
                agent_tools: vec!["run_tests".to_string()],
                display_order: 1,
                content_schema: None,
            },
        ]
    }

    // --- decomp_prompt tests ---

    #[test]
    fn decomp_prompt_includes_all_ports() {
        let ports = make_ports();
        let prompt = decomp_prompt(&ports);
        assert!(prompt.contains("Port \"frontend\""));
        assert!(prompt.contains("Port \"backend\""));
        assert!(prompt.contains("FE Agent"));
        assert!(prompt.contains("BE Agent"));
        assert!(prompt.contains("break it down"));
    }

    #[test]
    fn decomp_prompt_includes_multi_assignment_language() {
        let ports = make_ports();
        let prompt = decomp_prompt(&ports);
        assert!(prompt.contains("multiple subtasks to the same agent"));
        assert!(prompt.contains("execute independently"));
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
            content_schema: None,
        }];
        let prompt = decomp_prompt(&ports);
        assert!(!prompt.contains("Tools:"));
    }

    #[test]
    fn decomp_prompt_includes_calibration_guidance() {
        let ports = make_ports();
        let prompt = decomp_prompt(&ports);
        assert!(prompt.contains("fewest subtasks needed"));
        assert!(prompt.contains("independently"));
    }

    #[test]
    fn decomp_prompt_includes_dependency_guidance() {
        let ports = make_ports();
        let prompt = decomp_prompt(&ports);
        assert!(prompt.contains("ordering dependencies"));
    }

    // --- route_prompt tests ---

    #[test]
    fn route_prompt_includes_all_ports() {
        let ports = make_ports();
        let prompt = route_prompt(&ports);
        assert!(prompt.contains("Port \"frontend\""));
        assert!(prompt.contains("exactly one agent"));
    }

    #[test]
    fn route_prompt_includes_routing_criteria() {
        let ports = make_ports();
        let prompt = route_prompt(&ports);
        assert!(prompt.contains("core intent"));
        assert!(prompt.contains("best fit"));
    }

    // --- review_prompt tests ---

    #[test]
    fn review_prompt_includes_decisions() {
        let decisions = vec!["approve".to_string(), "reject".to_string()];
        let prompt = review_prompt(&decisions);
        assert!(prompt.contains("approve"));
        assert!(prompt.contains("reject"));
    }

    #[test]
    fn review_prompt_includes_evaluation_criteria() {
        let decisions = vec!["approve".to_string()];
        let prompt = review_prompt(&decisions);
        assert!(prompt.contains("Correctness"));
        assert!(prompt.contains("Completeness"));
        assert!(prompt.contains("Quality"));
    }

    #[test]
    fn review_prompt_includes_anti_rubber_stamping() {
        let decisions = vec!["approve".to_string()];
        let prompt = review_prompt(&decisions);
        assert!(prompt.contains("own independent assessment"));
        assert!(prompt.contains("Do not assume"));
    }

    // --- transform_prompt tests ---

    #[test]
    fn transform_prompt_includes_processing_steps() {
        let prompt = transform_prompt(None);
        assert!(prompt.contains("Identify the relevant data"));
        assert!(prompt.contains("Map each piece"));
    }

    #[test]
    fn transform_prompt_includes_positive_output_framing() {
        let prompt = transform_prompt(None);
        assert!(prompt.contains("parsed directly by a JSON parser"));
    }

    #[test]
    fn transform_prompt_appends_schema_context() {
        let prompt = transform_prompt(Some("A user profile object"));
        assert!(prompt.contains("Schema context: A user profile object"));
    }

    #[test]
    fn transform_prompt_omits_schema_context_when_none() {
        let prompt = transform_prompt(None);
        assert!(!prompt.contains("Schema context"));
    }

    // --- documenter_prompt tests ---

    fn make_doc_defs() -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({"name": "API Reference", "description": "REST API docs", "target_length": 5000}),
            serde_json::json!({"name": "Architecture Guide", "description": "System overview", "target_length": 3000}),
        ]
    }

    #[test]
    fn documenter_prompt_includes_all_documents() {
        let defs = make_doc_defs();
        let prompt = documenter_prompt(&defs, &[]);
        assert!(prompt.contains("\"API Reference\""));
        assert!(prompt.contains("\"Architecture Guide\""));
        assert!(prompt.contains("~5000 characters"));
        assert!(prompt.contains("~3000 characters"));
        assert!(prompt.contains("REST API docs"));
        assert!(prompt.contains("System overview"));
    }

    #[test]
    fn documenter_prompt_includes_strategist_role() {
        let defs = make_doc_defs();
        let prompt = documenter_prompt(&defs, &[]);
        assert!(prompt.contains("Document Strategist"));
    }

    #[test]
    fn documenter_prompt_includes_capabilities() {
        let defs = make_doc_defs();
        let caps = vec!["web_search".to_string(), "code_analysis".to_string()];
        let prompt = documenter_prompt(&defs, &caps);
        assert!(prompt.contains("Available Research Capabilities:"));
        assert!(prompt.contains("- web_search"));
        assert!(prompt.contains("- code_analysis"));
    }

    #[test]
    fn documenter_prompt_omits_capabilities_when_empty() {
        let defs = make_doc_defs();
        let prompt = documenter_prompt(&defs, &[]);
        assert!(!prompt.contains("Available Research Capabilities:"));
    }

    #[test]
    fn documenter_prompt_includes_response_format() {
        let defs = make_doc_defs();
        let prompt = documenter_prompt(&defs, &[]);
        assert!(prompt.contains("document_name"));
        assert!(prompt.contains("research_strategy"));
        assert!(prompt.contains("required_capabilities"));
        assert!(prompt.contains("writer_prompt"));
        assert!(prompt.contains("document_plans"));
    }

    #[test]
    fn documenter_prompt_handles_empty_description() {
        let defs =
            vec![serde_json::json!({"name": "Readme", "description": "", "target_length": 1000})];
        let prompt = documenter_prompt(&defs, &[]);
        assert!(prompt.contains("\"Readme\""));
        assert!(prompt.contains("~1000 characters"));
        // Should not have an em dash for empty description
        assert!(!prompt.contains("\"Readme\" —"));
    }
}
