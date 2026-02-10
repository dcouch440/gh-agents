//! DebateVerificationFilter — multi-agent critique panel for step outputs.
//!
//! When a workflow step has `verification_agent_ids` configured, this filter
//! runs each verification agent in parallel against the primary agent's response.
//! Each verification agent reviews the output from its area of expertise and
//! returns structured critique. If any agent raises issues, the primary agent
//! gets a retry with the merged feedback.
//!
//! Research backing:
//! - Du et al. "Improving Factuality and Reasoning through Multiagent Debate" — +4-6% factuality
//! - A-HMAD framework — heterogeneous agents (different roles/models) yield +9% accuracy
//! - Anthropic multi-agent research system — lead + sub-agents outperforms single by 90.2%
//!
//! No effect when `verification_agent_ids` is empty.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::constants::VERIFICATION_AGENT_TIMEOUT_SECS;
use crate::db::traits::{AgentExecutionRepo, ServerRepo, TokenLedgerRepo};
use crate::llm::{LLMProvider, LLMRequest, LLMResponse, Message, Role};
use crate::server::hub::strategies::compute_cost;

use super::{ExecutionFilter, FilterContext, HubError, ResponseAction};

/// Captures the original prompt context from `on_start` for use in `on_response`.
#[derive(Clone)]
pub(crate) struct PromptCapture {
    pub(crate) system_prompt: String,
    pub(crate) user_prompt: String,
}

/// Multi-agent critique panel that verifies the primary agent's output.
pub struct DebateVerificationFilter {
    provider: Arc<dyn LLMProvider>,
    repo: Arc<dyn ServerRepo>,
    verification_agent_ids: Vec<Uuid>,
    /// Captured from on_start for use in on_response.
    pub(crate) prompt_context: tokio::sync::Mutex<Option<PromptCapture>>,
    /// For recording verification agent executions (audit trail).
    ae_repo: Option<Arc<dyn AgentExecutionRepo>>,
    /// For recording token/cost usage of verification calls.
    tl_repo: Option<Arc<dyn TokenLedgerRepo>>,
}

/// Maximum tokens for a verification critique response.
const MAX_CRITIQUE_TOKENS: u32 = 1024;

/// Structured critique from a verification agent.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
struct CritiqueResponse {
    approved: bool,
    #[serde(default)]
    issues: Vec<CritiqueIssue>,
}

/// A single issue raised by a verification agent.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
struct CritiqueIssue {
    severity: String,
    description: String,
    #[serde(default)]
    suggestion: Option<String>,
}

/// Result of running a single verification agent.
struct VerificationResult {
    agent_name: String,
    critique: CritiqueResponse,
}

impl DebateVerificationFilter {
    pub fn new(
        provider: Arc<dyn LLMProvider>,
        repo: Arc<dyn ServerRepo>,
        verification_agent_ids: Vec<Uuid>,
        ae_repo: Option<Arc<dyn AgentExecutionRepo>>,
        tl_repo: Option<Arc<dyn TokenLedgerRepo>>,
    ) -> Self {
        Self {
            provider,
            repo,
            verification_agent_ids,
            prompt_context: tokio::sync::Mutex::new(None),
            ae_repo,
            tl_repo,
        }
    }

    /// Build the system prompt for a verification agent.
    fn build_verifier_system_prompt(agent_name: &str, agent_system_prompt: &str) -> String {
        format!(
            "{agent_system_prompt}\n\n\
             <verification_role>\n\
             You are a verification panelist reviewing another agent's output.\n\
             Your domain of expertise: {agent_name}.\n\n\
             Form your own independent assessment. Do not assume the response is correct.\n\n\
             <severity_definitions>\n\
             - high: Blocks correctness or introduces a significant defect. Must be fixed.\n\
             - medium: Reduces quality or omits important detail. Should be fixed.\n\
             - low: Style, convention, or minor improvement. Nice to fix.\n\
             </severity_definitions>\n\n\
             <evaluation_process>\n\
             1. Identify what the response does well in your domain.\n\
             2. Identify specific issues — cite exact quotes or references.\n\
             3. For each issue, classify severity and provide a concrete fix.\n\
             4. Set approved to true only if there are no high or medium severity issues.\n\
             </evaluation_process>\n\
             </verification_role>\n\n\
             Respond with this JSON structure:\n\
             {{\n  \
               \"approved\": false,\n  \
               \"issues\": [\n    \
                 {{\n      \
                   \"severity\": \"high\",\n      \
                   \"description\": \"Specific issue with a direct quote or reference\",\n      \
                   \"suggestion\": \"Concrete fix\"\n    \
                 }}\n  \
               ]\n\
             }}"
        )
    }

    /// Build the user message for a verification agent.
    fn build_verifier_user_message(
        system_prompt_summary: &str,
        user_prompt: &str,
        primary_response: &str,
    ) -> String {
        format!(
            "<original_task>\n\
             <context>\n{system_prompt_summary}\n</context>\n\
             <request>\n{user_prompt}\n</request>\n\
             </original_task>\n\n\
             <response_under_review>\n{primary_response}\n</response_under_review>\n\n\
             Evaluate this response for factual accuracy, logical consistency, \
             and completeness within your area of expertise."
        )
    }

    /// Format the merged critique feedback for the primary agent's retry.
    fn format_feedback(results: &[VerificationResult]) -> String {
        let mut feedback = String::from(
            "<verification_feedback>\n\
             Your response was reviewed by a panel of specialist agents. \
             Address HIGH severity issues first, then MEDIUM. \
             Retain aspects of your original response that were approved.\n",
        );

        for result in results {
            if result.critique.approved {
                feedback.push_str(&format!(
                    "\n<reviewer name=\"{}\" verdict=\"approved\">\n\
                     No issues found in their domain.\n\
                     </reviewer>\n",
                    result.agent_name
                ));
            } else {
                feedback.push_str(&format!(
                    "\n<reviewer name=\"{}\" verdict=\"needs_revision\">\n",
                    result.agent_name
                ));
                for issue in &result.critique.issues {
                    let severity = issue.severity.to_uppercase();
                    feedback.push_str(&format!("- [{}] {}", severity, issue.description));
                    if let Some(suggestion) = &issue.suggestion {
                        feedback.push_str(&format!(" → {}", suggestion));
                    }
                    feedback.push('\n');
                }
                feedback.push_str("</reviewer>\n");
            }
        }

        feedback.push_str("</verification_feedback>");
        feedback
    }
}

#[async_trait]
impl ExecutionFilter for DebateVerificationFilter {
    fn name(&self) -> &str {
        "debate_verification"
    }

    async fn on_start(
        &self,
        _ctx: &FilterContext,
        system_prompt: String,
        messages: Vec<Message>,
    ) -> Result<(String, Vec<Message>), HubError> {
        if self.verification_agent_ids.is_empty() {
            return Ok((system_prompt, messages));
        }

        // Extract the first user message as the prompt context.
        let user_prompt = messages
            .iter()
            .find(|m| m.role == Role::User)
            .map(|m| m.text())
            .unwrap_or_default();

        *self.prompt_context.lock().await = Some(PromptCapture {
            system_prompt: system_prompt.clone(),
            user_prompt,
        });

        Ok((system_prompt, messages))
    }

    async fn on_response(
        &self,
        ctx: &FilterContext,
        response: &LLMResponse,
    ) -> Result<ResponseAction, HubError> {
        if self.verification_agent_ids.is_empty() {
            return Ok(ResponseAction::Accept);
        }

        // Clone captured data and drop the guard immediately.
        let capture = {
            let guard = self.prompt_context.lock().await;
            match guard.as_ref() {
                Some(c) => c.clone(),
                None => {
                    warn!(
                        filter = "debate_verification",
                        "no prompt context captured, skipping"
                    );
                    return Ok(ResponseAction::Accept);
                }
            }
        };

        debug!(
            filter = "debate_verification",
            agent_count = self.verification_agent_ids.len(),
            agent_id = %ctx.agent_id,
            step_id = ?ctx.step_id,
            "running verification panel"
        );

        // Extract cross-cutting metadata from FilterContext.
        let parent_execution_id: Option<Uuid> = ctx
            .metadata
            .get("agent_execution_id")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let user_id: Option<Uuid> = ctx
            .metadata
            .get("user_id")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let workflow_execution_id: Option<Uuid> = ctx
            .metadata
            .get("workflow_execution_id")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let step_id = ctx.step_id;

        // Launch parallel critique tasks.
        let mut join_set = tokio::task::JoinSet::new();

        for &verifier_id in &self.verification_agent_ids {
            let provider = Arc::clone(&self.provider);
            let repo = Arc::clone(&self.repo);
            let ae_repo = self.ae_repo.clone();
            let tl_repo = self.tl_repo.clone();
            let system_prompt_summary = capture.system_prompt.clone();
            let user_prompt = capture.user_prompt.clone();
            let primary_response = response.content.clone();

            join_set.spawn(async move {
                // Load the verification agent.
                let agent = match repo.get_persisted_agent(verifier_id).await {
                    Ok(Some(agent)) => agent,
                    Ok(None) => {
                        warn!(verifier_id = %verifier_id, "verification agent not found, treating as approved");
                        return VerificationResult {
                            agent_name: format!("Unknown ({})", verifier_id),
                            critique: CritiqueResponse {
                                approved: true,
                                issues: vec![],
                            },
                        };
                    }
                    Err(e) => {
                        warn!(verifier_id = %verifier_id, error = %e, "failed to load verification agent");
                        return VerificationResult {
                            agent_name: format!("Unknown ({})", verifier_id),
                            critique: CritiqueResponse {
                                approved: true,
                                issues: vec![],
                            },
                        };
                    }
                };

                let verifier_system =
                    DebateVerificationFilter::build_verifier_system_prompt(&agent.name, &agent.system_prompt);
                let verifier_user = DebateVerificationFilter::build_verifier_user_message(
                    &system_prompt_summary,
                    &user_prompt,
                    &primary_response,
                );

                // Record verification execution (best-effort audit trail).
                let verification_ae_id = if let Some(ref ae_repo) = ae_repo {
                    match ae_repo
                        .create_agent_execution(
                            verifier_id,
                            step_id,
                            false,
                            parent_execution_id,
                            &verifier_system,
                            &verifier_user,
                            None,
                            None,
                            None,
                            workflow_execution_id,
                        )
                        .await
                    {
                        Ok(row) => Some(row.id),
                        Err(e) => {
                            warn!(verifier_id = %verifier_id, error = %e, "failed to record verification execution");
                            None
                        }
                    }
                } else {
                    None
                };

                let request = LLMRequest {
                    model: agent.model_id.clone(),
                    messages: vec![Message::user(&verifier_user)],
                    system: Some(verifier_system),
                    max_tokens: MAX_CRITIQUE_TOKENS,
                    temperature: agent.model_temperature,
                    tools: vec![],
                    stream: false,
                };

                // Execute with timeout.
                let llm_result = tokio::time::timeout(
                    Duration::from_secs(VERIFICATION_AGENT_TIMEOUT_SECS),
                    provider.send_message(request),
                )
                .await;

                let (critique, status) = match llm_result {
                    Ok(Ok(llm_response)) => {
                        // Record token usage (best-effort).
                        if let (Some(ref tl_repo), Some(uid)) = (&tl_repo, user_id) {
                            let cost = compute_cost(
                                &agent.model_id,
                                llm_response.usage.input_tokens as i64,
                                llm_response.usage.output_tokens as i64,
                            );
                            let _ = tl_repo
                                .insert_ledger_entry(
                                    uid,
                                    verification_ae_id,
                                    &agent.model_id,
                                    llm_response.usage.input_tokens as i64,
                                    llm_response.usage.output_tokens as i64,
                                    cost,
                                )
                                .await;
                        }

                        let critique = serde_json::from_str::<CritiqueResponse>(
                            &llm_response.content,
                        )
                        .or_else(|_| extract_json_from_response(&llm_response.content))
                        .unwrap_or_else(|_| {
                            warn!(
                                verifier = agent.name,
                                "unparseable critique, treating as approved"
                            );
                            CritiqueResponse {
                                approved: true,
                                issues: vec![],
                            }
                        });
                        (critique, "completed")
                    }
                    Ok(Err(e)) => {
                        warn!(
                            verifier = agent.name,
                            error = %e,
                            "verification agent LLM call failed, treating as approved"
                        );
                        (
                            CritiqueResponse {
                                approved: true,
                                issues: vec![],
                            },
                            "failed",
                        )
                    }
                    Err(_elapsed) => {
                        warn!(
                            verifier = agent.name,
                            timeout_secs = VERIFICATION_AGENT_TIMEOUT_SECS,
                            "verification agent timed out, treating as approved"
                        );
                        (
                            CritiqueResponse {
                                approved: true,
                                issues: vec![],
                            },
                            "timeout",
                        )
                    }
                };

                // Update execution record with result (best-effort).
                if let (Some(ref ae_repo), Some(ae_id)) = (&ae_repo, verification_ae_id) {
                    let output_json = serde_json::to_string(&critique).ok();
                    let _ = ae_repo
                        .update_agent_execution_status(ae_id, status, output_json, None)
                        .await;
                }

                VerificationResult {
                    agent_name: agent.name,
                    critique,
                }
            });
        }

        // Collect all results.
        let mut results = Vec::with_capacity(self.verification_agent_ids.len());
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(vr) => results.push(vr),
                Err(e) => {
                    warn!(error = %e, "verification task panicked, treating as approved");
                    results.push(VerificationResult {
                        agent_name: "Unknown (panicked)".to_string(),
                        critique: CritiqueResponse {
                            approved: true,
                            issues: vec![],
                        },
                    });
                }
            }
        }

        // Check if all approved.
        let all_approved = results.iter().all(|r| r.critique.approved);

        if all_approved {
            debug!(
                filter = "debate_verification",
                "all verification agents approved"
            );
            Ok(ResponseAction::Accept)
        } else {
            let feedback = Self::format_feedback(&results);
            debug!(
                filter = "debate_verification",
                "critique raised, requesting retry"
            );
            Ok(ResponseAction::Retry { feedback })
        }
    }
}

/// Try to extract JSON from a response that may be wrapped in markdown code blocks.
fn extract_json_from_response(content: &str) -> Result<CritiqueResponse, serde_json::Error> {
    // Look for ```json ... ``` blocks.
    if let Some(start) = content.find("```json") {
        let json_start = start + 7;
        if let Some(end) = content[json_start..].find("```") {
            let json_str = content[json_start..json_start + end].trim();
            return serde_json::from_str(json_str);
        }
    }
    // Look for ``` ... ``` blocks without language tag.
    if let Some(start) = content.find("```") {
        let json_start = start + 3;
        if let Some(end) = content[json_start..].find("```") {
            let json_str = content[json_start..json_start + end].trim();
            return serde_json::from_str(json_str);
        }
    }
    // Try the raw content as a last resort.
    serde_json::from_str(content)
}

#[cfg(test)]
mod tests;
