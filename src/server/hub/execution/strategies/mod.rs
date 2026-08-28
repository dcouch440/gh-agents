//! ExecutionStrategy implementations.

pub mod chat;
pub mod dag_step;
pub mod manager_dispatch;
pub mod system_node;
pub mod workflow_agent;
pub mod workforce_agent;

pub use chat::{ChatConfig, ChatStrategy, StepChatContext};
pub use dag_step::DagStepStrategy;
pub use manager_dispatch::ManagerDispatchStrategy;
pub use system_node::SystemNodeStrategy;
pub use workflow_agent::WorkflowAgentStrategy;
pub use workforce_agent::{WorkforceAgentConfig, WorkforceAgentStrategy};

use crate::db::ChatMessageRow;
use crate::llm::TokenUsage;
use crate::server::state::AppState;
use uuid::Uuid;

/// Build a single instruction string from session history + current instruction.
///
/// Drops all prior user messages (stale full-node before/after blocks) and keeps
/// the last `max_summaries` assistant messages (passdown summaries). These are
/// formatted as a numbered `<prior_work>` XML block prepended to the instruction.
///
/// The board_state in the system prompt is the source of truth for current node
/// state — prior user messages are redundant. The passdown summaries provide
/// lightweight continuity about what the builder previously configured.
pub fn build_pruned_instruction(
    history: &[ChatMessageRow],
    instruction: &str,
    max_summaries: usize,
) -> String {
    let assistant_summaries: Vec<&str> = history
        .iter()
        .filter(|row| row.role == "assistant")
        .map(|row| row.content.as_str())
        .collect();

    let keep = assistant_summaries.len().min(max_summaries);
    let recent = &assistant_summaries[assistant_summaries.len() - keep..];

    if recent.is_empty() {
        return instruction.to_string();
    }

    let mut prior_work = String::from("<prior_work>\n");
    for (i, summary) in recent.iter().enumerate() {
        prior_work.push_str(&format!("{}. {}\n", i + 1, summary));
    }
    prior_work.push_str("</prior_work>");

    format!("{}\n\n{}", prior_work, instruction)
}

/// Log token usage to the ledger. Shared by all strategies that track costs.
pub async fn log_token_usage(
    state: &AppState,
    user_id: Uuid,
    agent_execution_id: Option<Uuid>,
    model_id: &str,
    usage: &TokenUsage,
) {
    let cost = crate::server::hub::pricing::compute_cost_cached(
        model_id,
        usage.input_tokens as i64,
        usage.cached_input_tokens as i64,
        usage.output_tokens as i64,
    );
    let _ = state
        .repos()
        .token_ledger
        .insert_ledger_entry(
            user_id,
            agent_execution_id,
            model_id,
            usage.input_tokens as i64,
            usage.output_tokens as i64,
            cost,
        )
        .await;
}

/// Complete an agent execution: log token usage + update status.
///
/// Handles optional state/user_id/ae_id gracefully (skips if None).
/// When `parse_structured` is true, attempts to parse JSON from the response.
pub async fn complete_agent_execution(
    state: Option<&AppState>,
    user_id: Option<Uuid>,
    agent_execution_id: Option<Uuid>,
    model_id: &str,
    response: &str,
    usage: &TokenUsage,
    parse_structured: bool,
) {
    if let (Some(state), Some(uid)) = (state, user_id) {
        log_token_usage(state, uid, agent_execution_id, model_id, usage).await;
    }

    if let (Some(state), Some(ae_id)) = (state, agent_execution_id) {
        let structured = if parse_structured {
            crate::server::hub::protocols::json_utils::parse_structured_output(response)
        } else {
            None
        };
        let _ = state
            .repos()
            .agent_executions
            .update_agent_execution_status(
                ae_id,
                "completed",
                Some(response.to_string()),
                structured,
            )
            .await;
    }
}

/// Re-export compute_cost from hub::pricing for backward compatibility.
pub use crate::server::hub::pricing::compute_cost;
