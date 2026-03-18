//! Grok one-shot LLM conflict resolution.
//!
//! Sends conflict hunks with file-type-aware context to Grok for resolution.
//! Uses the `create_utility_client()` pattern from the distiller module.

use tracing::warn;

use crate::constants;
use crate::llm::{ContentBlock, LLMRequest, Message};

use super::types::{ConflictContext, ConflictHunk, FileType, StepInfo};
use super::verify::{verify_resolution, VerifyOutcome};

/// System prompt for the merge resolver.
const MERGE_SYSTEM_PROMPT: &str = "\
You are a code merge resolver. Two parallel agents independently modified \
the same file. Both agents' changes are intentional and must be preserved.

Rules:
1. PRESERVE both agents' changes. Never drop one agent's work.
2. For imports: include ALL imports from both versions.
3. For function bodies: integrate both changes into one coherent function. \
   If both add processing steps, chain them. If both add branches, keep both.
4. For config files: merge additively. Both agents' entries should appear.
5. For documentation: combine both perspectives into coherent prose.
6. Match the surrounding code style exactly (indentation, quotes, semicolons).
7. Output ONLY the merged content for the conflicting region. \
   No explanation. No markdown fences. No commentary.";

/// Resolve a single conflict hunk via Grok one-shot call.
///
/// Returns `(resolved_content, tokens_used)`.
/// Retries once on verification failure.
pub async fn resolve_hunk(
    hunk: &ConflictHunk,
    context: &ConflictContext,
    step_a: &StepInfo,
    step_b: &StepInfo,
) -> Result<(String, u64), String> {
    let prompt = build_hunk_prompt(hunk, context, step_a, step_b);

    // First attempt
    let (result, mut tokens) =
        call_grok(&prompt, constants::MERGE_MODEL, constants::MERGE_MAX_TOKENS).await?;

    // Verify
    match verify_resolution(
        &result,
        &hunk.version_a_lines,
        &hunk.version_b_lines,
        &context.file_type,
    ) {
        VerifyOutcome::Ok | VerifyOutcome::Warning(_) => Ok((result, tokens)),
        VerifyOutcome::Failed(reason) => {
            warn!(
                file = %context.file_path,
                reason = %reason,
                "Merge resolution failed verification — retrying"
            );

            // Retry with error context
            let retry_prompt = format!(
                "{}\n\nYour previous merge was invalid: {}. Try again, ensuring the output is valid.",
                prompt, reason
            );
            let (retry_result, retry_tokens) = call_grok(
                &retry_prompt,
                constants::MERGE_MODEL,
                constants::MERGE_MAX_TOKENS,
            )
            .await?;
            tokens += retry_tokens;

            match verify_resolution(
                &retry_result,
                &hunk.version_a_lines,
                &hunk.version_b_lines,
                &context.file_type,
            ) {
                VerifyOutcome::Ok | VerifyOutcome::Warning(_) => Ok((retry_result, tokens)),
                VerifyOutcome::Failed(reason) => {
                    // Give up — return both versions with conflict markers
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

/// Resolve a delete-modify conflict via Grok.
///
/// Returns `(keep_file, tokens_used)`.
pub async fn resolve_delete_modify(
    file_path: &str,
    deleter: &StepInfo,
    modifier: &StepInfo,
    diff_summary: &str,
) -> (bool, u64) {
    let prompt = format!(
        "File: {file_path}\n\n\
         Agent A ({name_a}: \"{desc_a}\") DELETED this file.\n\n\
         Agent B ({name_b}: \"{desc_b}\") MODIFIED this file:\n\
         --- Changes ---\n\
         {diff_summary}\n\n\
         Should this file be kept (with Agent B's modifications) or deleted?\n\
         Respond with exactly one word: KEEP or DELETE.",
        name_a = deleter.name,
        desc_a = deleter.description,
        name_b = modifier.name,
        desc_b = modifier.description,
    );

    match call_grok(&prompt, constants::MERGE_MODEL_COMPLEX, 16).await {
        Ok((response, tokens)) => {
            let trimmed = response.trim().to_uppercase();
            (!trimmed.contains("DELETE"), tokens)
        }
        Err(e) => {
            warn!(file = %file_path, error = %e, "Delete-modify LLM call failed — keeping file");
            (true, 0) // Default: keep the file
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
    let type_label = file_type_label(file_type);
    let prompt = format!(
        "File: {file_path}\nType: {type_label}\n\n\
         Two agents independently created this file. Merge them into one coherent file.\n\n\
         --- AGENT A ({name_a}: \"{desc_a}\") ---\n\
         {content_a}\n\n\
         --- AGENT B ({name_b}: \"{desc_b}\") ---\n\
         {content_b}\n\n\
         Produce the complete merged file.",
        name_a = step_a.name,
        desc_a = step_a.description,
        name_b = step_b.name,
        desc_b = step_b.description,
    );

    call_grok(
        &prompt,
        constants::MERGE_MODEL_COMPLEX,
        constants::MERGE_MAX_TOKENS,
    )
    .await
}

// ── Prompt Building ──────────────────────────────────────────────────────────

fn build_hunk_prompt(
    hunk: &ConflictHunk,
    context: &ConflictContext,
    step_a: &StepInfo,
    step_b: &StepInfo,
) -> String {
    let type_label = file_type_label(&context.file_type);
    let context_block = build_context_block(context);

    format!(
        "File: {} (lines {}-{})\nType: {}\n\n\
         {}\n\n\
         --- BASE (before either agent) ---\n\
         {}\n\n\
         --- AGENT A ({}: \"{}\") ---\n\
         {}\n\n\
         --- AGENT B ({}: \"{}\") ---\n\
         {}\n\n\
         Merge both agents' changes for this region.",
        context.file_path,
        hunk.base_line_range.start + 1,
        hunk.base_line_range.end,
        type_label,
        context_block,
        hunk.base_lines,
        step_a.name,
        step_a.description,
        hunk.version_a_lines,
        step_b.name,
        step_b.description,
        hunk.version_b_lines,
    )
}

fn build_context_block(context: &ConflictContext) -> String {
    let budget = constants::MERGE_MAX_CONTEXT_CHARS;
    let mut parts = Vec::new();

    if let Some(ref full_file) = context.full_file {
        let truncated = truncate_with_marker(full_file, budget);
        parts.push(format!("Full file:\n{}", truncated));
        return parts.join("\n\n");
    }

    // Priority order for truncation: imports > outline > scope > surrounding
    // We add in priority order and truncate from the bottom up if over budget.
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

/// Returns `(text_content, total_tokens)`.
async fn call_grok(prompt: &str, model: &str, max_tokens: u32) -> Result<(String, u64), String> {
    let client = crate::llm::create_utility_client()
        .map_err(|e| format!("Failed to create LLM client: {e}"))?;

    let request = LLMRequest {
        model: model.to_string(),
        system: Some(MERGE_SYSTEM_PROMPT.to_string()),
        messages: vec![Message::user(prompt)],
        max_tokens,
        temperature: constants::MERGE_TEMPERATURE,
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
