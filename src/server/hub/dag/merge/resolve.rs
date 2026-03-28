//! LLM conflict resolution via the config prompt system.
//!
//! Uses `RoleDefinition` templates from `config/services/merge/` with
//! `{{.Merge.*}}` injection sites. Model config from `config.yaml`.

use std::collections::HashMap;

use tracing::warn;

use crate::config::protocols::{roles, vars, AgentConfig, MERGE};
use crate::constants;
use crate::llm::{ContentBlock, LLMRequest, Message};

use super::types::{ConflictContext, ConflictHunk, FileType, StepInfo};
use super::verify::{verify_resolution, VerifyOutcome};

/// Resolve a single conflict hunk via one-shot LLM call.
///
/// Returns `(resolved_content, tokens_used)`.
/// Retries once on verification failure.
pub async fn resolve_hunk(
    hunk: &ConflictHunk,
    context: &ConflictContext,
    step_a: &StepInfo,
    step_b: &StepInfo,
) -> Result<(String, u64), String> {
    let context_block = build_context_block(context);
    let cfg = MERGE.agent("resolver");

    let mut vars = HashMap::new();
    vars.insert(
        vars::merge::FILE_PATH.to_string(),
        context.file_path.clone(),
    );
    vars.insert(
        vars::merge::LINE_RANGE.to_string(),
        format!(
            "{}-{}",
            hunk.base_line_range.start + 1,
            hunk.base_line_range.end
        ),
    );
    vars.insert(
        vars::merge::FILE_TYPE.to_string(),
        file_type_label(&context.file_type),
    );
    vars.insert(vars::merge::CONTEXT_BLOCK.to_string(), context_block);
    vars.insert(vars::merge::BASE_HUNK.to_string(), hunk.base_lines.clone());
    vars.insert(
        vars::merge::VERSION_A_HUNK.to_string(),
        hunk.version_a_lines.clone(),
    );
    vars.insert(
        vars::merge::VERSION_B_HUNK.to_string(),
        hunk.version_b_lines.clone(),
    );
    vars.insert(vars::merge::STEP_A_NAME.to_string(), step_a.name.clone());
    vars.insert(
        vars::merge::STEP_A_DESCRIPTION.to_string(),
        step_a.description.clone(),
    );
    vars.insert(vars::merge::STEP_B_NAME.to_string(), step_b.name.clone());
    vars.insert(
        vars::merge::STEP_B_DESCRIPTION.to_string(),
        step_b.description.clone(),
    );

    let resolved = roles::MERGE_HUNK.resolve(&vars);

    // First attempt
    let (result, mut tokens) =
        call_merge_llm(&resolved.system_prompt, &resolved.user_prompt, cfg).await?;

    // Verify
    match verify_resolution(
        &result,
        &hunk.version_a_lines,
        &hunk.version_b_lines,
        &context.file_type,
    ) {
        VerifyOutcome::Ok | VerifyOutcome::Warning => Ok((result, tokens)),
        VerifyOutcome::Failed(reason) => {
            warn!(
                file = %context.file_path,
                reason = %reason,
                "Merge resolution failed verification — retrying"
            );

            // Retry with error context appended to the user prompt
            let retry_prompt = format!(
                "{}\n\nYour previous merge was invalid: {}. Try again, ensuring the output is valid.",
                resolved.user_prompt, reason
            );
            let (retry_result, retry_tokens) =
                call_merge_llm(&resolved.system_prompt, &retry_prompt, cfg).await?;
            tokens += retry_tokens;

            match verify_resolution(
                &retry_result,
                &hunk.version_a_lines,
                &hunk.version_b_lines,
                &context.file_type,
            ) {
                VerifyOutcome::Ok | VerifyOutcome::Warning => Ok((retry_result, tokens)),
                VerifyOutcome::Failed(reason) => {
                    warn!(
                        file = %context.file_path,
                        reason = %reason,
                        "Merge resolution failed verification after retry — using conflict markers"
                    );
                    Ok((
                        format!(
                            "<<<<<<< {}\n{}\n=======\n{}\n>>>>>>> {}",
                            step_a.name, hunk.version_a_lines, hunk.version_b_lines, step_b.name
                        ),
                        tokens,
                    ))
                }
            }
        }
    }
}

/// Resolve a delete-modify conflict.
///
/// Returns `(keep_file, tokens_used)`.
pub async fn resolve_delete_modify(
    file_path: &str,
    deleter: &StepInfo,
    modifier: &StepInfo,
    diff_summary: &str,
) -> (bool, u64) {
    let cfg = MERGE.agent("complex_resolver");

    let mut vars = HashMap::new();
    vars.insert(vars::merge::FILE_PATH.to_string(), file_path.to_string());
    vars.insert(vars::merge::STEP_A_NAME.to_string(), deleter.name.clone());
    vars.insert(
        vars::merge::STEP_A_DESCRIPTION.to_string(),
        deleter.description.clone(),
    );
    vars.insert(vars::merge::STEP_B_NAME.to_string(), modifier.name.clone());
    vars.insert(
        vars::merge::STEP_B_DESCRIPTION.to_string(),
        modifier.description.clone(),
    );
    vars.insert(
        vars::merge::DIFF_SUMMARY.to_string(),
        diff_summary.to_string(),
    );

    let resolved = roles::MERGE_DELETE_MODIFY.resolve(&vars);

    match call_merge_llm(&resolved.system_prompt, &resolved.user_prompt, cfg).await {
        Ok((response, tokens)) => {
            let trimmed = response.trim().to_uppercase();
            (!trimmed.contains("DELETE"), tokens)
        }
        Err(e) => {
            warn!(file = %file_path, error = %e, "Delete-modify LLM call failed — keeping file");
            (true, 0)
        }
    }
}

/// Resolve a new-new conflict (both agents created the same file).
///
/// Returns `(merged_content, tokens_used)`.
pub async fn resolve_new_new(
    file_path: &str,
    file_type: &FileType,
    step_a: &StepInfo,
    content_a: &str,
    step_b: &StepInfo,
    content_b: &str,
) -> Result<(String, u64), String> {
    let cfg = MERGE.agent("complex_resolver");

    let mut vars = HashMap::new();
    vars.insert(vars::merge::FILE_PATH.to_string(), file_path.to_string());
    vars.insert(
        vars::merge::FILE_TYPE.to_string(),
        file_type_label(file_type),
    );
    vars.insert(vars::merge::STEP_A_NAME.to_string(), step_a.name.clone());
    vars.insert(
        vars::merge::STEP_A_DESCRIPTION.to_string(),
        step_a.description.clone(),
    );
    vars.insert(vars::merge::STEP_B_NAME.to_string(), step_b.name.clone());
    vars.insert(
        vars::merge::STEP_B_DESCRIPTION.to_string(),
        step_b.description.clone(),
    );
    vars.insert(vars::merge::CONTENT_A.to_string(), content_a.to_string());
    vars.insert(vars::merge::CONTENT_B.to_string(), content_b.to_string());

    let resolved = roles::MERGE_NEW_NEW.resolve(&vars);

    call_merge_llm(&resolved.system_prompt, &resolved.user_prompt, cfg).await
}

// ── Context Assembly ─────────────────────────────────────────────────────────

/// Build the context block from a `ConflictContext` struct.
///
/// This assembles the value for `{{.Merge.context_block}}`. Stays in code
/// because it's algorithmic (priority-based truncation), not a template.
pub(crate) fn build_context_block(context: &ConflictContext) -> String {
    let budget = constants::MERGE_MAX_CONTEXT_CHARS;
    let mut parts = Vec::new();

    if let Some(ref full_file) = context.full_file {
        let truncated = truncate_with_marker(full_file, budget);
        parts.push(format!("Full file:\n{}", truncated));
        return parts.join("\n\n");
    }

    // Priority order for truncation: imports > outline > scope > surrounding
    if let Some(ref imports) = context.import_block {
        parts.push(format!(
            "Imports (combined from all versions):\n{}",
            imports
        ));
    }

    if let Some(ref outline) = context.document_outline {
        parts.push(format!("Document structure:\n{}", outline));
    }

    if let Some(ref scope) = context.enclosing_scope {
        parts.push(format!(
            "Enclosing {} \"{}\" (line {}):\n{}",
            scope.kind,
            scope.name,
            scope.start_line + 1,
            scope.content
        ));
    }

    if !context.surrounding_lines.is_empty() {
        parts.push(format!(
            "Surrounding context:\n{}",
            context.surrounding_lines
        ));
    }

    // Enforce budget — trim from the end (lowest priority) first
    let mut result = parts.join("\n\n");
    if result.len() > budget {
        while result.len() > budget && parts.len() > 1 {
            parts.pop();
            result = parts.join("\n\n");
        }
        if result.len() > budget {
            result = truncate_with_marker(&result, budget);
        }
    }

    result
}

fn truncate_with_marker(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    format!("{}\n[... truncated]", &s[..max_chars])
}

fn file_type_label(file_type: &FileType) -> String {
    match file_type {
        FileType::Code(lang) => format!("Code ({:?})", lang),
        FileType::Markup(kind) => format!("Markup ({:?})", kind),
        FileType::Structured(kind) => format!("Structured ({:?})", kind),
        FileType::Config => "Config".to_string(),
        FileType::Binary => "Binary".to_string(),
        FileType::Unknown => "Unknown".to_string(),
    }
}

// ── LLM Call ─────────────────────────────────────────────────────────────────

/// Send a one-shot merge resolution request to the configured LLM provider.
///
/// Returns `(text_content, total_tokens)`.
async fn call_merge_llm(
    system_prompt: &str,
    user_prompt: &str,
    cfg: &AgentConfig,
) -> Result<(String, u64), String> {
    let client = crate::llm::create_utility_client()
        .map_err(|e| format!("Failed to create LLM client: {e}"))?;

    let request = LLMRequest {
        model: cfg.model_id.clone(),
        system: Some(system_prompt.to_string()),
        messages: vec![Message::user(user_prompt)],
        max_tokens: cfg.max_tokens,
        temperature: cfg.temperature,
        ..Default::default()
    };

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(constants::MERGE_LLM_TIMEOUT_SECS),
        client.send_message(request),
    )
    .await
    .map_err(|_| "LLM call timed out".to_string())?
    .map_err(|e| format!("LLM call failed: {e}"))?;

    let tokens = response.usage.total() as u64;

    let text = response
        .content_blocks
        .first()
        .and_then(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .ok_or_else(|| "LLM returned no text content".to_string())?;

    Ok((text, tokens))
}
