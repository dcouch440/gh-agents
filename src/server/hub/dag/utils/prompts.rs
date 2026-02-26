//! Prompt composition: build full prompts with context, documents, and routing.

use std::collections::HashMap;

use serde_json::Value as JsonValue;

use crate::db::traits::{AgentRepo, DocumentRepo, PromptTemplateRepo, WorkflowRepo};
use crate::db::WorkflowStepRow;
use crate::types::DownstreamRoutingContext;

use super::variables::resolve_variables;

/// Repository references needed for prompt composition.
pub(crate) struct PromptRepos<'a> {
    pub prompt_template_repo: Option<&'a dyn PromptTemplateRepo>,
    pub doc_repo: Option<&'a dyn DocumentRepo>,
    pub workflow_repo: Option<&'a dyn WorkflowRepo>,
    pub agent_repo: &'a dyn AgentRepo,
}

/// Build the full prompt for a step execution.
///
/// Resolves the prompt template, appends attached document content.
/// If `port_inputs` is provided, port values are injected as structured context
/// and made available for `{port_name}` variable resolution.
pub(crate) async fn compose_prompt(
    step: &WorkflowStepRow,
    repos: &PromptRepos<'_>,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
    port_inputs: Option<&HashMap<String, JsonValue>>,
) -> String {
    // Get prompt text: prefer saved template, fall back to inline
    let raw_prompt = if let Some(pt_id) = step.prompt_template_id {
        if let Some(repo) = repos.prompt_template_repo {
            repo.get_prompt_template(pt_id)
                .await
                .ok()
                .flatten()
                .map(|pt| pt.content)
                .unwrap_or_else(|| step.prompt_template.clone())
        } else {
            step.prompt_template.clone()
        }
    } else {
        step.prompt_template.clone()
    };

    // If no prompt template exists, construct one from available inputs so data flows
    // through without requiring explicit templates on every step.
    let raw_prompt = if raw_prompt.is_empty() {
        let mut parts: Vec<String> = Vec::new();
        // Port inputs from upstream steps (wired via edges)
        if let Some(ports) = port_inputs {
            for value in ports.values() {
                match value {
                    JsonValue::String(s) => parts.push(s.clone()),
                    other => parts.push(
                        serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
                    ),
                }
            }
        }
        // Prior outputs (port of entry / collection pipeline)
        if parts.is_empty() {
            for value in prior_outputs.values() {
                match value {
                    JsonValue::String(s) => parts.push(s.clone()),
                    other => parts.push(
                        serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
                    ),
                }
            }
        }
        // Last resort: inject completed upstream step variable outputs
        if parts.is_empty() {
            for value in outputs.values() {
                match value {
                    JsonValue::String(s) => parts.push(s.clone()),
                    other => parts.push(
                        serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
                    ),
                }
            }
        }
        parts.join("\n\n")
    } else {
        raw_prompt
    };

    // Merge port inputs into the variable resolution scope so {port_name} works
    let effective_outputs = if let Some(ports) = port_inputs {
        if !ports.is_empty() {
            let mut merged = outputs.clone();
            for (k, v) in ports {
                merged.insert(k.clone(), v.clone());
            }
            merged
        } else {
            outputs.clone()
        }
    } else {
        outputs.clone()
    };

    let prompt = resolve_variables(&raw_prompt, &effective_outputs, prior_outputs);

    let mut full_prompt = format!("<task>\n{}\n</task>", prompt);

    // Inject board context (annotations, sketches from the canvas)
    if !step.board_context_cache.is_empty() {
        full_prompt.push_str(&format!(
            "\n\n<board_context>\n{}\n</board_context>",
            step.board_context_cache
        ));
    }

    // Track whether we've opened a <context> block (for port inputs + documents).
    let mut context_opened = false;

    // Append structured port input data block — only for ports NOT referenced in the template.
    // If the user wrote {port_name} or {port_name.field}, the data is already inlined.
    if let Some(ports) = port_inputs {
        let unreferenced: Vec<_> = ports
            .iter()
            .filter(|(name, _)| !raw_prompt.contains(&format!("{{{}", name)))
            .collect();
        if !unreferenced.is_empty() {
            full_prompt.push_str("\n\n<context>");
            context_opened = true;
            for (port_name, value) in unreferenced {
                let formatted =
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
                full_prompt.push_str(&format!(
                    "\n<input name=\"{}\">\n{}\n</input>",
                    port_name, formatted
                ));
            }
        }
    }

    // Append agent context documents (global to agent)
    if let Some(_d_repo) = repos.doc_repo {
        if let Some(agent_id) = step.agent_id {
            if let Ok(agent_docs) = repos.agent_repo.get_agent_context(agent_id).await {
                if !agent_docs.is_empty() && !context_opened {
                    full_prompt.push_str("\n\n<context>");
                    context_opened = true;
                }
                for doc in &agent_docs {
                    let short_id = &doc.id.to_string()[..8];
                    full_prompt.push_str(&format!(
                        "\n<document_{short_id} title=\"{}\" source=\"agent\">\n{}\n</document_{short_id}>",
                        doc.title, doc.content
                    ));
                }
            }
        }
    }

    // Append step documents (specific to this workflow step)
    if let Some(wf_repo) = repos.workflow_repo {
        if let Ok(step_docs) = wf_repo.list_step_documents(step.id).await {
            if let Some(d_repo) = repos.doc_repo {
                let mut step_doc_contents = Vec::new();
                for sd in &step_docs {
                    if let Ok(Some(doc)) = d_repo.get_document(sd.document_id).await {
                        step_doc_contents.push(doc);
                    }
                }
                if !step_doc_contents.is_empty() && !context_opened {
                    full_prompt.push_str("\n\n<context>");
                    context_opened = true;
                }
                for doc in &step_doc_contents {
                    let short_id = &doc.id.to_string()[..8];
                    full_prompt.push_str(&format!(
                        "\n<document_{short_id} title=\"{}\" source=\"step\">\n{}\n</document_{short_id}>",
                        doc.title, doc.content
                    ));
                }
            }
        }
    }

    // Close context block if it was opened
    if context_opened {
        full_prompt.push_str("\n</context>");
    }

    full_prompt
}

/// Build a routing instruction text block from downstream routing context.
///
/// Appended to a planner step's prompt to inform the LLM about valid label
/// values, their meanings, and which agents handle each route.
pub(crate) fn build_routing_instruction_block(ctx: &DownstreamRoutingContext) -> String {
    let mut block = String::new();

    block.push_str("\n\n<routing>\n");
    block.push_str(&format!(
        "Each item MUST include a \"{}\" field set to exactly one of the following values.\n",
        ctx.routing_field
    ));
    block.push_str("Pick the single best match for each item. Do not use any other values.\n\n");

    for route in &ctx.routes {
        block.push_str(&format!("- {}", route.label_value));

        if let Some(ref desc) = route.description {
            block.push_str(&format!(": {}", desc));
        }

        block.push('\n');

        if route.agent_tools.is_empty() {
            block.push_str(&format!("  Routed to: {} (no tools)\n\n", route.agent_name));
        } else {
            let tools_str = route.agent_tools.join(", ");
            block.push_str(&format!(
                "  Routed to: {} (tools: {})\n\n",
                route.agent_name, tools_str
            ));
        }
    }

    block.push_str("</routing>");

    block
}
