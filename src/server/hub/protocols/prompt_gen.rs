//! Prompt injection auto-generation utilities for protocols.
//!
//! These prompts are appended to `step.prompt_template` and end up inside
//! `<task>` tags at runtime. The output schema is enforced separately via
//! `<schema>` in the system prompt, so these provide semantic guidance rather
//! than hard format constraints.
//!
//! Each protocol prompt is defined as a readable `const` template with
//! `{{.Protocol.field}}` placeholders, resolved at expansion time via
//! [`super::template_resolve::resolve_template`].

use std::collections::HashMap;

use super::template_resolve::resolve_template;
use super::types::PortConfig;

// ============================================================================
// Template constants
// ============================================================================

const DECOMP_TEMPLATE: &str = "\
Analyze the task above and break it down into subtasks \
for the specialist agents below.

Guidelines:
- Decompose into the fewest subtasks needed. \
Each should be a self-contained unit of work.
- Provide each agent with enough context to execute independently \
\u{2014} they cannot see each other's work.
- You may assign multiple subtasks to the same agent \
when the work covers distinct concerns within their expertise.
- If subtasks have ordering dependencies, \
note them in the content field.

Available Agents:
{{.Protocol.available_agents}}

Respond with a JSON array. Each element has a \"port\" field \
matching an agent identifier above, and a \"content\" field \
with the task details for that agent.";

const ROUTE_TEMPLATE: &str = "\
Analyze the input above to determine which specialist agent \
is the best fit, then route it to exactly one agent.

Routing criteria:
- Identify the core intent and requirements of the input.
- Match those requirements against each agent's \
expertise and available tools.
- Select the single best match. If multiple agents could handle \
the input, choose the one whose expertise most directly \
addresses the primary need.

Available Agents:
{{.Protocol.available_agents}}

Respond with a JSON object containing a \"port\" field \
matching an agent identifier above, and a \"content\" field \
with the task details for that agent.";

const REVIEW_TEMPLATE: &str = "\
Evaluate the input above and provide your assessment.

Evaluation criteria:
- Correctness: Does the content accurately fulfill \
the original requirements?
- Completeness: Are all expected elements present, \
or is anything missing?
- Quality: Is the output well-structured, clear, \
and free of obvious errors?

Form your own independent assessment. Do not assume the input \
is correct \u{2014} verify claims and check for gaps.

Your decision must be one of: \"{{.Protocol.decisions}}\"

Respond with a JSON object containing:
- \"decision\": one of the valid decisions listed above
- \"feedback\": a specific explanation citing what works, \
what does not, and what to change. Reference concrete details \
rather than giving generic praise or criticism.";

const TRANSFORM_TEMPLATE: &str = "\
Transform the input above into the structured format \
defined by the output schema.

Steps:
1. Identify the relevant data from the input.
2. Map each piece of data to the corresponding field \
in the schema.
3. Ensure all required fields are populated \
and values match the expected types.

Your response is parsed directly by a JSON parser \
\u{2014} output only the JSON object.
{{.Protocol.schema_context}}";

const DOCUMENTER_TEMPLATE: &str = "\
You are a Document Strategist. Your job is to plan how each \
requested document should be researched and written.

Requested Documents:
{{.Protocol.requested_documents}}

{{.Protocol.available_capabilities}}

For each document, provide:
- document_name: must match one of the document names listed above exactly
- research_strategy: a step-by-step plan for gathering the information \
needed to write this document
- required_capabilities: which capabilities the researcher needs \
from the list above (empty array if no research tools are needed)
- writer_prompt: detailed instructions for the writer, including \
tone, structure, target audience, and focus areas

Respond with a JSON object containing a \"document_plans\" array \
with one entry per document.";

// ============================================================================
// Public prompt generators
// ============================================================================

/// Generate the decomp prompt injection: instructs the orchestrator to analyze
/// the task, break it into subtasks, and assign each to a specialist agent.
pub fn decomp_prompt(ports: &[PortConfig]) -> String {
    let mut vars = HashMap::new();
    vars.insert(
        "Protocol.available_agents".to_string(),
        format_agents_block(ports),
    );
    collapse_blank_lines(&resolve_template(DECOMP_TEMPLATE, &vars))
}

/// Generate the route prompt injection: instructs the orchestrator to analyze
/// the input and route it to exactly one specialist agent.
pub fn route_prompt(ports: &[PortConfig]) -> String {
    let mut vars = HashMap::new();
    vars.insert(
        "Protocol.available_agents".to_string(),
        format_agents_block(ports),
    );
    collapse_blank_lines(&resolve_template(ROUTE_TEMPLATE, &vars))
}

/// Generate the review prompt injection: instructs the agent to evaluate input
/// against quality criteria and provide a structured decision with feedback.
pub fn review_prompt(decisions: &[String]) -> String {
    let mut vars = HashMap::new();
    vars.insert(
        "Protocol.decisions".to_string(),
        format_decisions(decisions),
    );
    collapse_blank_lines(&resolve_template(REVIEW_TEMPLATE, &vars))
}

/// Generate the transform prompt injection: instructs the agent to transform
/// the input into a structured format matching the output schema.
pub fn transform_prompt(schema_description: Option<&str>) -> String {
    let mut vars = HashMap::new();
    vars.insert(
        "Protocol.schema_context".to_string(),
        format_schema_context(schema_description),
    );
    collapse_blank_lines(&resolve_template(TRANSFORM_TEMPLATE, &vars))
}

/// Generate the documenter prompt injection: instructs the strategist to plan
/// research and writing for each requested document.
pub fn documenter_prompt(doc_defs: &[serde_json::Value], capabilities: &[String]) -> String {
    let mut vars = HashMap::new();
    vars.insert(
        "Protocol.requested_documents".to_string(),
        format_documents_block(doc_defs),
    );
    vars.insert(
        "Protocol.available_capabilities".to_string(),
        format_capabilities_block(capabilities),
    );
    collapse_blank_lines(&resolve_template(DOCUMENTER_TEMPLATE, &vars))
}

// ============================================================================
// Formatter helpers — build substitution values for template placeholders
// ============================================================================

/// Format the agent/port listing used by decomp and route templates.
fn format_agents_block(ports: &[PortConfig]) -> String {
    let mut parts = Vec::new();
    for port in ports {
        parts.push(String::new());
        parts.push(format!(
            "Port \"{}\" \u{2014} {}",
            port.port_name, port.agent_name
        ));
        parts.push(format!("  {}", port.description));
        if !port.agent_tools.is_empty() {
            parts.push(format!("  Tools: {}", port.agent_tools.join(", ")));
        }
    }
    parts.join("\n")
}

/// Format the decision list for the review template.
fn format_decisions(decisions: &[String]) -> String {
    decisions.join("\", \"")
}

/// Format the optional schema context block for the transform template.
fn format_schema_context(desc: Option<&str>) -> String {
    match desc {
        Some(d) => format!("\nSchema context: {}", d),
        None => String::new(),
    }
}

/// Format the numbered document listing for the documenter template.
fn format_documents_block(doc_defs: &[serde_json::Value]) -> String {
    let mut parts = Vec::new();
    for (i, def) in doc_defs.iter().enumerate() {
        let name = def["name"].as_str().unwrap_or("Unnamed");
        let description = def["description"].as_str().unwrap_or("");
        let target_length = def["target_length"].as_i64().unwrap_or(2000);

        parts.push(String::new());
        if description.is_empty() {
            parts.push(format!(
                "{}. \"{}\" (target: ~{} characters)",
                i + 1,
                name,
                target_length
            ));
        } else {
            parts.push(format!(
                "{}. \"{}\" \u{2014} {} (target: ~{} characters)",
                i + 1,
                name,
                description,
                target_length
            ));
        }
    }
    parts.join("\n")
}

/// Format the capabilities block for the documenter template.
/// Returns empty string when no capabilities are available.
fn format_capabilities_block(capabilities: &[String]) -> String {
    if capabilities.is_empty() {
        return String::new();
    }
    let mut parts = vec!["Available Research Capabilities:".to_string()];
    for cap in capabilities {
        parts.push(format!("- {}", cap));
    }
    parts.join("\n")
}

// ============================================================================
// Whitespace utility
// ============================================================================

/// Collapse consecutive blank lines into a single blank line
/// and trim trailing blank lines.
fn collapse_blank_lines(s: &str) -> String {
    let mut result = Vec::new();
    let mut prev_blank = false;
    for line in s.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        result.push(line);
        prev_blank = is_blank;
    }
    while result.last().is_some_and(|l| l.trim().is_empty()) {
        result.pop();
    }
    result.join("\n")
}

// ============================================================================
// Tests
// ============================================================================

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
        assert!(!prompt.contains("\"Readme\" \u{2014}"));
    }

    // --- collapse_blank_lines tests ---

    #[test]
    fn collapse_blank_lines_removes_consecutive_blanks() {
        let input = "a\n\n\nb";
        assert_eq!(collapse_blank_lines(input), "a\n\nb");
    }

    #[test]
    fn collapse_blank_lines_preserves_single_blanks() {
        let input = "a\n\nb";
        assert_eq!(collapse_blank_lines(input), "a\n\nb");
    }

    #[test]
    fn collapse_blank_lines_trims_trailing_blanks() {
        let input = "a\n\n";
        assert_eq!(collapse_blank_lines(input), "a");
    }

    #[test]
    fn collapse_blank_lines_empty_input() {
        assert_eq!(collapse_blank_lines(""), "");
    }

    // --- formatter tests ---

    #[test]
    fn format_agents_block_includes_tools() {
        let ports = make_ports();
        let block = format_agents_block(&ports);
        assert!(block.contains("Port \"frontend\" \u{2014} FE Agent"));
        assert!(block.contains("Tools: read_file, write_file"));
    }

    #[test]
    fn format_agents_block_omits_empty_tools() {
        let ports = vec![PortConfig {
            port_name: "worker".to_string(),
            description: "General worker".to_string(),
            agent_id: Uuid::new_v4(),
            agent_name: "Worker".to_string(),
            agent_tools: vec![],
            display_order: 0,
            content_schema: None,
        }];
        let block = format_agents_block(&ports);
        assert!(!block.contains("Tools:"));
        assert!(block.contains("Port \"worker\" \u{2014} Worker"));
    }

    #[test]
    fn format_capabilities_block_empty_returns_empty() {
        assert_eq!(format_capabilities_block(&[]), "");
    }

    #[test]
    fn format_capabilities_block_with_caps() {
        let caps = vec!["search".to_string(), "code".to_string()];
        let block = format_capabilities_block(&caps);
        assert!(block.contains("Available Research Capabilities:"));
        assert!(block.contains("- search"));
        assert!(block.contains("- code"));
    }

    #[test]
    fn format_schema_context_none_returns_empty() {
        assert_eq!(format_schema_context(None), "");
    }

    #[test]
    fn format_schema_context_some_includes_prefix() {
        let ctx = format_schema_context(Some("User profile"));
        assert!(ctx.contains("Schema context: User profile"));
    }
}
