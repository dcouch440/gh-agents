//! Haiku distiller functions for board context.
//!
//! Two distillation functions:
//! - `distill_board_for_node()` — per-node perspective summary of the full board
//! - `distill_node_goal()` — conversational intent summary for a single node

use crate::llm::{AnthropicClient, AnthropicConfig, LLMProvider, LLMRequest, Message as LlmMessage};

/// Max tokens for the board context distillation response.
const MAX_TOKENS_BOARD_CONTEXT: u32 = 512;

/// Max tokens for the goal summary response.
const MAX_TOKENS_GOAL_SUMMARY: u32 = 128;

/// Distill the full board render into targeted context for a specific node.
///
/// Haiku reads the full board document and produces a 3-5 sentence summary
/// from the perspective of the specified node — what neighboring nodes are
/// working on, how this node fits into the user's broader design, and any
/// user emphasis visible from connected nodes.
pub async fn distill_board_for_node(
    board_render: &str,
    node_name: &str,
    node_archetype: &str,
) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let user_text = format!(
        "<board>\n{}</board>\n\n<target_node>\n{} ({})\n</target_node>",
        board_render, node_name, node_archetype
    );

    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(user_text)],
    )
    .with_system(BOARD_DISTILLER_SYSTEM_PROMPT)
    .with_max_tokens(MAX_TOKENS_BOARD_CONTEXT);

    match client.send_message(request).await {
        Ok(resp) => {
            let text = resp.content.trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        Err(e) => {
            tracing::warn!("Board context distillation failed: {}", e);
            None
        }
    }
}

/// Distill a node's conversational intent into a 1-2 sentence goal summary.
///
/// Captures WHAT the user wants and WHY — not just the task description.
/// The goal is used in the board renderer for neighbor awareness.
pub async fn distill_node_goal(
    recent_conversation: &str,
    node_name: &str,
    node_archetype: &str,
    current_goal: &str,
) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let user_text = format!(
        "<node>\n{} ({})\n</node>\n\n<current_goal>\n{}\n</current_goal>\n\n<conversation>\n{}\n</conversation>",
        node_name, node_archetype,
        if current_goal.is_empty() { "None yet" } else { current_goal },
        recent_conversation
    );

    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(user_text)],
    )
    .with_system(GOAL_DISTILLER_SYSTEM_PROMPT)
    .with_max_tokens(MAX_TOKENS_GOAL_SUMMARY);

    match client.send_message(request).await {
        Ok(resp) => {
            let text = resp.content.trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        Err(e) => {
            tracing::warn!("Goal distillation failed: {}", e);
            None
        }
    }
}

const BOARD_DISTILLER_SYSTEM_PROMPT: &str = r#"Summarize this workflow board from the perspective of the specified target node.
Write 3-5 sentences that help this node's assistant understand:
- What neighboring nodes are working on and why the user set them up
- How this node fits into the user's broader design
- Any user emphasis or priorities visible from connected nodes

Write in second person as a brief from a colleague. Be specific about
what matters to THIS node given its role and connections. Sound informed,
not clinical.

<examples>
<example>
<node>Doc Gen (documenter)</node>
<context>Your upstream Research Task Force is actively being configured — the user has invested significant effort there, defining a research team focused on agent entertainment behavior with an emphasis on real data over speculation. Your downstream Quality Review is set up as a security-first review room where the Security Lead has primary authority. Your specifications will be evaluated through a security compliance lens, so technical accuracy and threat surface coverage matter.</context>
</example>

<example>
<node>Quality Review (room)</node>
<context>You're receiving input from two sources: the Doc Gen documenter upstream is producing technical specifications, and the Research Task Force is feeding in behavioral analysis data. The user set up your meeting as a security-first review — the Security Lead has been positioned with primary authority. Both upstream nodes are actively being configured, so expect their output focus to sharpen as the user refines their goals.</context>
</example>
</examples>

Return only the context summary."#;

const GOAL_DISTILLER_SYSTEM_PROMPT: &str = r#"Distill this node's purpose from the user's conversation into 1-2 sentences.
Capture what the user is trying to accomplish AND their reasoning or emphasis.
Write as if briefing a colleague: "Focused on X because Y" not "This node does X."

<examples>
<example>
<conversation>User asked to set up a research team to investigate how agents behave during idle time, specifically entertainment. Added a Researcher agent focused on behavioral patterns and an Analyst for data synthesis. User emphasized wanting "real behavioral data, not speculation."</conversation>
<goal>Investigating agent entertainment behavior with emphasis on real behavioral data over speculation. Team is research-heavy with dedicated analysis capacity.</goal>
</example>

<example>
<conversation>User set up a security-focused review room. Added Security Lead first, then Tech Lead and Architect. Set interaction mode to moderated. User said "security is the lens everything goes through."</conversation>
<goal>Security-first architecture review where compliance is the primary lens. User prioritized the security perspective above other concerns.</goal>
</example>
</examples>

If the conversation is too early to determine intent, return: "Still being defined by the user"
Return only the goal statement."#;
