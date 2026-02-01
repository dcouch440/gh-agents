//! True Context distiller — a cheap LLM pre-pass that summarises recent
//! conversation into structured `<ctx_scop>` (technical intent) and
//! `<ctx_vibe>` (user sentiment) tags for injection into the agent's
//! system prompt.
//!
//! The prompt template is loaded from the agent's context documents
//! (`doc_type = "distiller"`). When no document is attached, a sensible
//! default template is used instead.

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::constants;
use crate::db::DocumentRow;
use crate::llm::{AnthropicClient, AnthropicConfig, ContentBlock, LLMProvider, LLMRequest, Message};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Structured output of the True Context distiller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrueContext {
    /// Technical scope: what the user is asking for and why this approach.
    pub scope: String,
    /// Application vibe: user intent, frustration, urgency, repeated asks.
    pub vibe: String,
}

/// The `doc_type` value used to identify distiller prompt templates.
pub const DISTILLER_DOC_TYPE: &str = "distiller";

// ---------------------------------------------------------------------------
// Default prompt template (fallback when no DB document is attached)
// ---------------------------------------------------------------------------

const DEFAULT_DISTILLER_PROMPT: &str = "\
You are a context distiller. Given recent conversation messages and a current task, \
produce a brief structured summary.

<ctx_scop>: In 1-2 sentences, describe what the user technically needs and why. \
Focus on the specific problem, what approach fits, and any constraints mentioned.

<ctx_vibe>: In 1-2 sentences, describe the user's underlying intent and tone. \
Are they frustrated? Repeating themselves? Exploring? In a rush? What do they \
actually mean beyond the literal words?

Recent messages:
{messages}

Current task:
{task_title}: {task_description}

Respond with ONLY this format, no other text:
<ctx_scop>...</ctx_scop>
<ctx_vibe>...</ctx_vibe>";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Distill true context from recent messages and the current task.
///
/// Looks for a `doc_type = "distiller"` document in `context_docs` for the
/// prompt template. Falls back to [`DEFAULT_DISTILLER_PROMPT`] if none found.
///
/// The LLM model is controlled by [`constants::DISTILLER_MODEL`] — set it to
/// a Grok model ID to route through web search, or any Anthropic model for a
/// cheap local call.
///
/// Returns `None` if the API key is missing, the call fails, or parsing fails —
/// the caller should proceed without context rather than blocking.
pub async fn distill_true_context(messages: &[Message], task_title: &str, task_description: &str, context_docs: &[DocumentRow]) -> Option<TrueContext> {
    let template = find_distiller_template(context_docs);
    let formatted_messages = format_messages(messages);

    let prompt = template
        .replace("{messages}", &formatted_messages)
        .replace("{task_title}", task_title)
        .replace("{task_description}", task_description);

    let response_text = if constants::DISTILLER_MODEL.starts_with("grok") {
        distill_via_grok(&prompt).await?
    } else {
        distill_via_anthropic(&prompt).await?
    };

    match parse_true_context(&response_text) {
        Some(ctx) => Some(ctx),
        None => {
            warn!(
                raw = %response_text,
                "true-context distiller: failed to parse ctx_scop/ctx_vibe tags"
            );
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Template resolution
// ---------------------------------------------------------------------------

/// Find the distiller prompt template from context documents.
/// Returns the document content if found, otherwise the default template.
fn find_distiller_template(docs: &[DocumentRow]) -> String {
    docs.iter()
        .find(|d| d.doc_type == DISTILLER_DOC_TYPE)
        .map(|d| d.content.clone())
        .unwrap_or_else(|| DEFAULT_DISTILLER_PROMPT.to_string())
}

// ---------------------------------------------------------------------------
// Provider backends
// ---------------------------------------------------------------------------

async fn distill_via_anthropic(prompt: &str) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let request = LLMRequest {
        model: constants::DISTILLER_MODEL.to_string(),
        system: None,
        messages: vec![Message::user(prompt)],
        max_tokens: constants::DISTILLER_MAX_TOKENS,
        temperature: constants::DISTILLER_TEMPERATURE,
        ..Default::default()
    };

    let response = match client.send_message(request).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "true-context distiller: Anthropic call failed");
            return None;
        }
    };

    response.content_blocks.first().and_then(|b| match b {
        ContentBlock::Text { text } => Some(text.clone()),
        _ => None,
    })
}

async fn distill_via_grok(prompt: &str) -> Option<String> {
    use crate::llm::{GrokResearchClient, ResearchRequest};

    let client = match GrokResearchClient::from_env() {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "true-context distiller: Grok client init failed");
            return None;
        }
    };

    let request = ResearchRequest {
        query: prompt.to_string(),
        sources: vec![],
        web_filters: None,
        x_filters: None,
        system_prompt: None,
    };

    match client.research(&request).await {
        Ok(res) => Some(res.answer),
        Err(e) => {
            warn!(error = %e, "true-context distiller: Grok research call failed");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Message formatting
// ---------------------------------------------------------------------------

fn format_messages(messages: &[Message]) -> String {
    if messages.is_empty() {
        return "(no prior messages)".to_string();
    }

    // Take last N messages
    let recent = if messages.len() > constants::DISTILLER_MAX_MESSAGES {
        &messages[messages.len() - constants::DISTILLER_MAX_MESSAGES..]
    } else {
        messages
    };

    let mut out = String::new();
    let mut chars_remaining = constants::DISTILLER_MAX_INPUT_CHARS;

    for msg in recent {
        let role_str = match msg.role {
            crate::llm::Role::User => "user",
            crate::llm::Role::Assistant => "assistant",
        };
        let content_str = match &msg.content {
            crate::llm::MessageContent::Text(t) => t.clone(),
            crate::llm::MessageContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" "),
        };
        let line = format!("[{role_str}]: {content_str}\n");
        if line.len() > chars_remaining {
            out.push_str(&line[..chars_remaining]);
            break;
        }
        chars_remaining -= line.len();
        out.push_str(&line);
    }

    out
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

fn parse_true_context(text: &str) -> Option<TrueContext> {
    let scope = extract_between(text, "<ctx_scop>", "</ctx_scop>")?;
    let vibe = extract_between(text, "<ctx_vibe>", "</ctx_vibe>")?;

    let scope = scope.trim();
    let vibe = vibe.trim();

    if scope.is_empty() || vibe.is_empty() {
        return None;
    }

    Some(TrueContext {
        scope: scope.to_string(),
        vibe: vibe.to_string(),
    })
}

fn extract_between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)? + open.len();
    let end = text[start..].find(close)? + start;
    Some(&text[start..end])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_doc(doc_type: &str, content: &str) -> DocumentRow {
        DocumentRow {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            session_id: None,
            title: "test".to_string(),
            content: content.to_string(),
            summary: String::new(),
            doc_type: doc_type.to_string(),
            ref_tag: String::new(),
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // -- Template resolution -------------------------------------------------

    #[test]
    fn find_distiller_template_from_docs() {
        let custom = "Custom prompt: {messages}\n{task_title}: {task_description}";
        let docs = vec![make_doc("architecture", "some arch doc"), make_doc("distiller", custom)];
        assert_eq!(find_distiller_template(&docs), custom);
    }

    #[test]
    fn find_distiller_template_fallback() {
        let docs = vec![make_doc("architecture", "some arch doc")];
        assert_eq!(find_distiller_template(&docs), DEFAULT_DISTILLER_PROMPT);
    }

    #[test]
    fn find_distiller_template_empty_docs() {
        let docs: Vec<DocumentRow> = vec![];
        assert_eq!(find_distiller_template(&docs), DEFAULT_DISTILLER_PROMPT);
    }

    // -- Parsing -------------------------------------------------------------

    #[test]
    fn parse_true_context_valid() {
        let input = "<ctx_scop>User needs a REST endpoint for uploads.</ctx_scop>\n\
                      <ctx_vibe>Exploratory tone, no urgency.</ctx_vibe>";
        let ctx = parse_true_context(input).unwrap();
        assert_eq!(ctx.scope, "User needs a REST endpoint for uploads.");
        assert_eq!(ctx.vibe, "Exploratory tone, no urgency.");
    }

    #[test]
    fn parse_true_context_with_surrounding_text() {
        let input = "Here is the analysis:\n\
                      <ctx_scop>Fix the login bug.</ctx_scop>\n\
                      <ctx_vibe>User is frustrated, third time asking.</ctx_vibe>\n\
                      Done.";
        let ctx = parse_true_context(input).unwrap();
        assert_eq!(ctx.scope, "Fix the login bug.");
        assert_eq!(ctx.vibe, "User is frustrated, third time asking.");
    }

    #[test]
    fn parse_true_context_missing_both_tags() {
        assert!(parse_true_context("no tags here").is_none());
    }

    #[test]
    fn parse_true_context_missing_vibe() {
        assert!(parse_true_context("<ctx_scop>Something.</ctx_scop>").is_none());
    }

    #[test]
    fn parse_true_context_missing_scope() {
        assert!(parse_true_context("<ctx_vibe>Something.</ctx_vibe>").is_none());
    }

    #[test]
    fn parse_true_context_empty_content() {
        let input = "<ctx_scop></ctx_scop>\n<ctx_vibe>ok</ctx_vibe>";
        assert!(parse_true_context(input).is_none());
    }

    // -- Extract helper ------------------------------------------------------

    #[test]
    fn extract_between_basic() {
        assert_eq!(extract_between("a<b>hello</b>c", "<b>", "</b>"), Some("hello"));
    }

    #[test]
    fn extract_between_missing_close() {
        assert_eq!(extract_between("a<b>hello", "<b>", "</b>"), None);
    }

    #[test]
    fn extract_between_missing_open() {
        assert_eq!(extract_between("hello</b>", "<b>", "</b>"), None);
    }

    // -- Message formatting --------------------------------------------------

    #[test]
    fn format_messages_empty() {
        assert_eq!(format_messages(&[]), "(no prior messages)");
    }

    #[test]
    fn format_messages_respects_max_count() {
        let msgs: Vec<Message> = (0..30).map(|i| Message::user(format!("msg {i}"))).collect();
        let result = format_messages(&msgs);
        let line_count = result.lines().count();
        assert!(
            line_count <= constants::DISTILLER_MAX_MESSAGES,
            "got {line_count} lines, expected <= {}",
            constants::DISTILLER_MAX_MESSAGES
        );
    }

    #[test]
    fn format_messages_respects_char_limit() {
        let msgs: Vec<Message> = (0..5).map(|_| Message::user("x".repeat(2000))).collect();
        let result = format_messages(&msgs);
        assert!(
            result.len() <= constants::DISTILLER_MAX_INPUT_CHARS + 50,
            "got {} chars, expected <= ~{}",
            result.len(),
            constants::DISTILLER_MAX_INPUT_CHARS
        );
    }

    // -- Constants -----------------------------------------------------------

    #[test]
    fn distiller_model_default_is_haiku() {
        assert_eq!(constants::DISTILLER_MODEL, constants::MODEL_HAIKU);
    }

    #[test]
    fn distiller_model_grok_routing() {
        assert!("grok-4-1-fast".starts_with("grok"));
        assert!(!"claude-3-5-haiku-20241022".starts_with("grok"));
    }
}
