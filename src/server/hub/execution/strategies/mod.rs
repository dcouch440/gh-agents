//! ExecutionStrategy implementations.

pub mod agent_designer;
pub mod board_dispatch;
pub mod chat;
pub mod dag_step;
pub mod dispatch;
pub mod manager_dispatch;
pub mod workforce_agent;

pub use agent_designer::{AgentDesignerConfig, AgentDesignerStrategy};
pub use board_dispatch::BoardDispatchStrategy;
pub use chat::{ChatConfig, ChatStrategy, StepChatContext};
pub use dag_step::DagStepStrategy;
pub use dispatch::DispatchStrategy;
pub use manager_dispatch::ManagerDispatchStrategy;
pub use workforce_agent::{WorkforceAgentConfig, WorkforceAgentStrategy};

use crate::llm::TokenUsage;
use crate::server::state::AppState;
use uuid::Uuid;

/// Log token usage to the ledger. Shared by all strategies that track costs.
pub async fn log_token_usage(
    state: &AppState,
    user_id: Uuid,
    agent_execution_id: Option<Uuid>,
    model_id: &str,
    usage: &TokenUsage,
) {
    let cost = compute_cost(
        model_id,
        usage.input_tokens as i64,
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
