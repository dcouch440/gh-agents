//! Downstream routing context assembly for label-based step routing.
//!
//! Builds routing context that gets injected into planner step prompts,
//! informing the LLM about valid label values and their target agents.

use std::collections::{HashMap, HashSet};

use tracing::warn;
use uuid::Uuid;

use crate::db::{StepRoutingRuleRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::state::AppState;
use crate::types::{DownstreamRoutingContext, RouteDescription};

use super::{get_child_steps, PortMetadata};

/// For a given step, find downstream label-routing steps and build
/// routing context for prompt injection.
///
/// Uses edges (in memory), the step map, and pre-fetched routing rules.
/// Batch-loads agent names and tools in two queries (regardless of rule count).
pub(crate) async fn gather_downstream_routing_context(
    step_id: Uuid,
    edges: &[WorkflowStepEdgeRow],
    step_map: &HashMap<Uuid, &WorkflowStepRow>,
    port_meta: &PortMetadata,
    state: &AppState,
) -> Vec<DownstreamRoutingContext> {
    let child_step_ids = get_child_steps(step_id, edges);

    // 1. Filter qualifying children and collect unique agent_ids
    let mut all_agent_ids: HashSet<Uuid> = HashSet::new();
    let mut qualifying: Vec<(Uuid, &str, &[StepRoutingRuleRow])> = Vec::new();

    for child_id in child_step_ids {
        let Some(child_step) = step_map.get(&child_id) else {
            continue;
        };
        if child_step.routing_mode.as_deref() != Some("label") {
            continue;
        }
        let Some(routing_field) = child_step.routing_field.as_deref() else {
            continue;
        };
        let Some(rules) = port_meta.routing_rules.get(&child_id) else {
            continue;
        };
        if rules.is_empty() {
            continue;
        }

        for rule in rules {
            all_agent_ids.insert(rule.agent_id);
        }
        qualifying.push((child_id, routing_field, rules));
    }

    if qualifying.is_empty() {
        return vec![];
    }

    let all_agent_ids: Vec<Uuid> = all_agent_ids.into_iter().collect();

    // 2. Batch fetch agents + tools (2 queries total)
    let agent_map: HashMap<Uuid, String> = state
        .repos()
        .agents
        .get_agents_by_ids(&all_agent_ids)
        .await
        .inspect_err(
            |e| warn!(step_id = %step_id, "Failed to fetch agents for routing context: {e}"),
        )
        .unwrap_or_default()
        .into_iter()
        .map(|a| (a.id, a.name))
        .collect();

    let mut tools_map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (agent_id, tool) in state
        .repos()
        .tools
        .get_tools_for_agents(&all_agent_ids)
        .await
        .inspect_err(
            |e| warn!(step_id = %step_id, "Failed to fetch tools for routing context: {e}"),
        )
        .unwrap_or_default()
    {
        tools_map.entry(agent_id).or_default().push(tool.name);
    }

    // 3. Assemble from maps (zero DB calls)
    qualifying
        .into_iter()
        .map(|(child_id, routing_field, rules)| {
            let routes = rules
                .iter()
                .map(|rule| RouteDescription {
                    label_value: rule.label_value.clone(),
                    description: rule.description.clone(),
                    agent_name: agent_map
                        .get(&rule.agent_id)
                        .cloned()
                        .unwrap_or_else(|| format!("Agent {}", rule.agent_id)),
                    agent_tools: tools_map.get(&rule.agent_id).cloned().unwrap_or_default(),
                })
                .collect();

            DownstreamRoutingContext {
                downstream_step_id: child_id,
                routing_field: routing_field.to_string(),
                routes,
            }
        })
        .collect()
}
