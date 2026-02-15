//! Documenter archetype formatters for the Agent Designer.
//!
//! The documenter has a three-phase pipeline (Strategy → Research → Write).
//! Two designer calls are needed:
//! 1. Before the strategist runs (1 agent)
//! 2. After the strategist produces document plans (N researchers + N writers)

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::{ProtocolDocumentDefRow, WorkflowStepRow};
use crate::server::hub::dag::documenter::types::DocumentPlan;
use crate::types::StepExecutionEnvelope;

use super::{
    build_tool_descriptions, format_envelopes_as_upstream, AgentDefinition, DesignerInput,
};

/// Build `DesignerInput` for the strategist (Phase 1).
///
/// Called BEFORE the strategist runs. Produces input for a single agent
/// that plans research strategies and writing instructions per document.
pub fn build_strategist_designer_input(
    _step: &WorkflowStepRow,
    doc_defs: &[ProtocolDocumentDefRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    available_capabilities: &[String],
    steps: &[WorkflowStepRow],
) -> DesignerInput {
    let docs_summary = doc_defs
        .iter()
        .map(|d| {
            format!(
                "- {} (~{} words): {}",
                d.name, d.target_length, d.description,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let agent = AgentDefinition {
        id: Uuid::new_v4().to_string(),
        name: "Document Strategist".to_string(),
        role: "Analyzes the task and upstream context to produce a research strategy \
               and detailed writing instructions for each requested document."
            .to_string(),
        capabilities: vec![],
        execution_order: 0,
        additional_context: format!(
            "Requested documents:\n{}\n\n\
             The strategist's output is a structured JSON with document_plans containing \
             research_strategy, required_capabilities, and writer_prompt per document. \
             This output directly drives the research and writing phases.",
            docs_summary,
        ),
    };

    DesignerInput {
        archetype: "documenter".to_string(),
        context_description: format!(
            "A documenter node producing {} reference documents. The strategist plans \
             research strategies and writing instructions for each document.",
            doc_defs.len(),
        ),
        agents: vec![agent],
        upstream: format_envelopes_as_upstream(completed_envelopes, steps),
        available_tools: build_tool_descriptions(available_capabilities),
        archetype_guidance: format!(
            "This is Phase 1 of a three-phase documenter pipeline:\n\
             Phase 1 (Strategist): Plans research and writing — this is the agent being designed\n\
             Phase 2 (Researchers): Execute the strategist's research plans using tools\n\
             Phase 3 (Writers): Produce final documents from research findings\n\n\
             The strategist must produce a JSON response with a document_plans array. \
             Each plan needs: document_name, research_strategy, required_capabilities, \
             writer_prompt, and optional context_document_ids.\n\n\
             Documents being produced:\n{}",
            docs_summary,
        ),
    }
}

/// Build `DesignerInput` for researchers + writers (Phase 2 & 3).
///
/// Called AFTER the strategist runs, using its document_plans output.
/// Generates agents for ALL documents in one call:
/// - One researcher per document (execution_order 0..N-1)
/// - One writer per document (execution_order N..2N-1)
pub fn build_research_write_designer_input(
    _step: &WorkflowStepRow,
    document_plans: &[DocumentPlan],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    available_capabilities: &[String],
    steps: &[WorkflowStepRow],
) -> DesignerInput {
    let mut agents = Vec::with_capacity(document_plans.len() * 2);

    // Researchers — one per document
    for (idx, plan) in document_plans.iter().enumerate() {
        agents.push(AgentDefinition {
            id: format!("researcher:{}", plan.document_name),
            name: format!("Researcher: {}", plan.document_name),
            role: format!(
                "Gathers information for the document '{}' using available tools.",
                plan.document_name,
            ),
            capabilities: plan.required_capabilities.clone(),
            execution_order: idx as i32,
            additional_context: format!(
                "Research strategy from the strategist:\n{}\n\n\
                 This researcher's findings will be passed to the writer. \
                 Summarize findings clearly — the writer depends on comprehensive, \
                 well-organized research output.",
                plan.research_strategy,
            ),
        });
    }

    // Writers — one per document, ordered after all researchers
    let researcher_count = document_plans.len();
    for (idx, plan) in document_plans.iter().enumerate() {
        agents.push(AgentDefinition {
            id: format!("writer:{}", plan.document_name),
            name: format!("Writer: {}", plan.document_name),
            role: format!(
                "Produces the final document '{}' from research findings.",
                plan.document_name,
            ),
            capabilities: vec![],
            execution_order: (researcher_count + idx) as i32,
            additional_context: format!(
                "Writing instructions from the strategist:\n{}\n\n\
                 The researcher's findings will be provided as input. \
                 Produce a well-structured, comprehensive document in markdown format.",
                plan.writer_prompt,
            ),
        });
    }

    DesignerInput {
        archetype: "documenter".to_string(),
        context_description: format!(
            "Phase 2 & 3 of a documenter pipeline. The strategist has produced plans \
             for {} documents. Researchers gather information, then writers produce \
             final documents. Researchers and writers execute in parallel within their phase.",
            document_plans.len(),
        ),
        agents,
        upstream: format_envelopes_as_upstream(completed_envelopes, steps),
        available_tools: build_tool_descriptions(available_capabilities),
        archetype_guidance: "Researchers run in parallel (Phase 2), then writers run in parallel \
             (Phase 3).\n\
             Each researcher's output feeds into the corresponding writer.\n\
             Researchers have tools; writers do not — they synthesize from research findings.\n\n\
             The strategist has already planned the research strategies and writing instructions. \
             Each agent's additional_context contains the strategist's specific guidance for them. \
             The designer should enrich the prompts with identity specificity and domain awareness \
             while preserving the strategist's intent."
            .to_string(),
    }
}
