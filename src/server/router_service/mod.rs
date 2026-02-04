//! Tool Router Service — routes user intents to tools via LLM-based routing.
//!
//! The router loads its configuration (system prompt, model, assigned tools),
//! builds a routing prompt with tool specs and conversation context, calls the
//! router LLM, and either executes the chosen tool inline (sync) or spawns a
//! background task and returns a passdown message (async).

use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::{ContextStoreRepo, RouterRequestRepo, ToolRouterRepo};
use crate::llm::{LLMProvider, LLMRequest, Message, Tool};
use crate::server::tools::filtered_tools;

// ── Public types ────────────────────────────────────────────────────────────

/// The result of routing a user intent.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteResult {
    /// Tool was executed inline; result is ready.
    Sync { result: String },
    /// Tool is executing in the background; passdown is a message for the user.
    Async { passdown: String, request_id: Uuid },
    /// Router decided no tool is needed.
    NoAction { reason: String },
}

/// Parsed JSON response from the router LLM.
#[derive(Debug, Clone, Deserialize)]
struct RouterDecision {
    tool: Option<String>,
    tool_args: Option<Value>,
    #[serde(default)]
    is_async: bool,
    passdown: Option<String>,
    chain: Option<Vec<ChainStep>>,
    reason: Option<String>,
}

/// A step in a multi-tool chain (future use).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStep {
    pub tool: String,
    pub args: Value,
}

// ── Service ─────────────────────────────────────────────────────────────────

/// Stateless service that performs LLM-based tool routing.
pub struct RouterService {
    tool_router_repo: Arc<dyn ToolRouterRepo>,
    context_repo: Arc<dyn ContextStoreRepo>,
    request_repo: Arc<dyn RouterRequestRepo>,
    llm: Arc<dyn LLMProvider>,
}

impl RouterService {
    pub fn new(
        tool_router_repo: Arc<dyn ToolRouterRepo>,
        context_repo: Arc<dyn ContextStoreRepo>,
        request_repo: Arc<dyn RouterRequestRepo>,
        llm: Arc<dyn LLMProvider>,
    ) -> Self {
        Self {
            tool_router_repo,
            context_repo,
            request_repo,
            llm,
        }
    }

    /// Route a user intent through a specific tool router.
    ///
    /// 1. Loads the router config and its assigned tools.
    /// 2. Builds a routing prompt with tool specs + conversation context.
    /// 3. Calls the router LLM → parses the JSON decision.
    /// 4. Logs the decision to `router_requests`.
    /// 5. Returns a `RouteResult`.
    pub async fn route_request(
        &self,
        router_id: Uuid,
        session_id: Uuid,
        agent_execution_id: Option<Uuid>,
        intent: &str,
        conversation_context: &str,
        priority: &str,
    ) -> Result<RouteResult> {
        // 1. Load router config + tools
        let router = self
            .tool_router_repo
            .get_tool_router(router_id)
            .await
            .context("failed to load tool router")?
            .ok_or_else(|| anyhow::anyhow!("tool router {} not found", router_id))?;

        // Load tool name keys from the DB, then resolve to actual Tool definitions
        // from the codebase. The DB join table is just an allow-list of names.
        let tool_rows = self
            .tool_router_repo
            .get_router_tools(router_id)
            .await
            .context("failed to load router tool assignments")?;
        let allowed_names: Vec<String> = tool_rows.iter().map(|r| r.name.clone()).collect();
        let tools = filtered_tools(&allowed_names);

        // 2. Build the routing prompt
        let tool_specs = build_tool_specs(&tools);
        let user_prompt = build_routing_prompt(intent, conversation_context, &tool_specs);

        // 3. Call the router LLM
        let request = LLMRequest::new(&router.model_id, vec![Message::user(&user_prompt)])
            .with_system(&router.system_prompt)
            .with_max_tokens(crate::constants::DEFAULT_MAX_TOKENS_WORKER);

        let response = self
            .llm
            .send_message(request)
            .await
            .map_err(|e| anyhow::anyhow!("router LLM call failed: {}", e))?;

        // 4. Parse decision
        let decision = parse_router_decision(&response.content)?;

        // 5. Log to router_requests
        let req_row = self
            .request_repo
            .create_router_request(
                session_id,
                agent_execution_id,
                intent,
                priority,
                None, // callback_hint
            )
            .await
            .context("failed to create router request")?;

        // Update with routing decision
        let status_str = if decision.tool.is_some() {
            "routed"
        } else {
            "no_action"
        };
        let chain_val = decision
            .chain
            .as_ref()
            .and_then(|c| serde_json::to_value(c).ok());
        self.request_repo
            .update_router_request(
                req_row.id,
                decision.tool.clone(),
                decision.tool_args.clone(),
                decision.is_async,
                decision.passdown.clone(),
                chain_val,
                status_str,
                decision.reason.clone(),
            )
            .await
            .context("failed to update router request")?;

        // 6. Build result
        match &decision.tool {
            Some(_tool_name) => {
                if decision.is_async {
                    let passdown = decision
                        .passdown
                        .unwrap_or_else(|| "Working on it...".to_string());
                    Ok(RouteResult::Async {
                        passdown,
                        request_id: req_row.id,
                    })
                } else {
                    // Sync execution — the actual tool execution is handled by the
                    // caller (execution engine / tool_router agent). We return the
                    // decision so the caller can dispatch.
                    Ok(RouteResult::Sync {
                        result: serde_json::to_string(&json!({
                            "tool": decision.tool,
                            "tool_args": decision.tool_args,
                            "request_id": req_row.id,
                        }))?,
                    })
                }
            }
            None => Ok(RouteResult::NoAction {
                reason: decision
                    .reason
                    .unwrap_or_else(|| "No tool matched the intent.".to_string()),
            }),
        }
    }

    /// Store a tool execution result back into the context store so the session
    /// can reference it in future turns.
    pub async fn store_result_as_context(
        &self,
        session_id: Uuid,
        request_id: Uuid,
        tool_name: &str,
        result: &str,
    ) -> Result<()> {
        let source = format!("router:{}:{}", tool_name, request_id);
        self.context_repo
            .add_context(
                session_id,
                &source,
                0.8, // high priority — fresh tool results
                result,
                Some(json!({ "request_id": request_id, "tool": tool_name })),
                None, // no expiry
            )
            .await
            .context("failed to store result in context store")?;

        // Mark request as completed
        self.request_repo
            .update_router_request(
                request_id,
                None,
                None,
                false,
                None,
                None,
                "completed",
                Some(truncate(result, 1000)),
            )
            .await
            .context("failed to mark request completed")?;

        Ok(())
    }

    /// Load active context entries for a session, ordered by priority.
    pub async fn load_session_context(&self, session_id: Uuid, limit: u32) -> Result<String> {
        // Expire stale entries first
        let expired = self
            .context_repo
            .expire_stale_context(session_id)
            .await
            .unwrap_or(0);
        if expired > 0 {
            tracing::debug!(session_id = %session_id, expired, "expired stale context entries");
        }

        let entries = self
            .context_repo
            .get_active_context(session_id, limit)
            .await
            .context("failed to load session context")?;

        if entries.is_empty() {
            return Ok(String::new());
        }

        let mut ctx = String::from("<session_context>\n");
        for entry in &entries {
            ctx.push_str(&format!(
                "[{}] (priority={:.1}) {}\n",
                entry.source, entry.priority, entry.content
            ));
        }
        ctx.push_str("</session_context>");
        Ok(ctx)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Build a human-readable tool spec block for the routing prompt.
fn build_tool_specs(tools: &[Tool]) -> String {
    if tools.is_empty() {
        return "No tools available.".to_string();
    }

    let mut specs = String::new();
    for tool in tools {
        specs.push_str(&format!(
            "- **{}**: {}\n  Parameters: {}\n",
            tool.name,
            tool.description,
            serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default()
        ));
    }
    specs
}

/// Build the full routing prompt sent to the router LLM.
fn build_routing_prompt(intent: &str, context: &str, tool_specs: &str) -> String {
    format!(
        r#"You are a tool router. Given the user's intent and available tools, decide which tool to call.

## Available Tools
{tool_specs}

## Conversation Context
{context}

## User Intent
{intent}

## Instructions
Respond with a JSON object:
{{
  "tool": "<tool_name or null if no tool matches>",
  "tool_args": {{ ... }},
  "is_async": false,
  "passdown": "<message for user if async, null if sync>",
  "chain": null,
  "reason": "<brief explanation of your routing decision>"
}}

Only use tools from the list above. If none match, set "tool" to null."#
    )
}

/// Parse the router LLM response into a structured decision.
fn parse_router_decision(content: &str) -> Result<RouterDecision> {
    // Try to extract JSON from the response (may be wrapped in markdown code block)
    let json_str = extract_json(content);
    serde_json::from_str::<RouterDecision>(&json_str)
        .context("failed to parse router decision JSON")
}

/// Extract JSON from a string that may contain markdown code fences.
fn extract_json(content: &str) -> String {
    // Try ```json ... ``` first
    if let Some(start) = content.find("```json") {
        if let Some(end) = content[start + 7..].find("```") {
            return content[start + 7..start + 7 + end].trim().to_string();
        }
    }
    // Try ``` ... ```
    if let Some(start) = content.find("```") {
        if let Some(end) = content[start + 3..].find("```") {
            return content[start + 3..start + 3 + end].trim().to_string();
        }
    }
    // Try raw JSON (find first { to last })
    if let Some(start) = content.find('{') {
        if let Some(end) = content.rfind('}') {
            return content[start..=end].to_string();
        }
    }
    content.to_string()
}

/// Truncate a string to a max length.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
