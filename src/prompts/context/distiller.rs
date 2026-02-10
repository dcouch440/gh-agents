//! True Context distiller — a cheap LLM pre-pass that summarises recent
//! conversation into structured context tags for injection into the agent's
//! system prompt.
//!
//! Tag names are **dynamic** — defined by a YAML front matter block in the
//! distiller document stored in the database (`doc_type = "distiller"`).
//! When no document is attached, a sensible default template with `scope`
//! and `vibe` tags is used instead.
//!
//! ## Document format
//!
//! ```text
//! ---
//! tags: [scope, vibe, urgency]
//! ---
//! You are a context distiller...
//! <scope>: describe technical need...
//! <vibe>: describe user tone...
//! <urgency>: describe time pressure...
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::constants;
use crate::db::DocumentRow;
use crate::llm::{
    AnthropicClient, AnthropicConfig, ContentBlock, LLMProvider, LLMRequest, Message,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Structured output of the True Context distiller.
///
/// Fields are dynamic — the keys come from the distiller document's front
/// matter `tags` list. When no front matter is present, defaults to
/// `scope` and `vibe`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrueContext {
    /// Dynamic key-value pairs extracted from the distiller LLM output.
    pub fields: HashMap<String, String>,
}

/// The `doc_type` value used to identify distiller prompt templates.
pub const DISTILLER_DOC_TYPE: &str = "distiller";

/// Default tag names when no front matter is present.
const DEFAULT_TAGS: &[&str] = &["scope", "vibe"];

// ---------------------------------------------------------------------------
// Default prompt template (fallback when no DB document is attached)
// ---------------------------------------------------------------------------

const DEFAULT_DISTILLER_PROMPT: &str = "\
You are a context distiller. Given recent conversation messages and a current task, \
produce a brief structured summary.

<scope>: In 1-2 sentences, describe what the user technically needs and why. \
Focus on the specific problem, what approach fits, and any constraints mentioned.

<vibe>: In 1-2 sentences, describe the user's underlying intent and tone. \
Are they frustrated? Repeating themselves? Exploring? In a rush? What do they \
actually mean beyond the literal words?

Recent messages:
{messages}

Current task:
{task_title}: {task_description}

Respond with ONLY this format, no other text:
<scope>...</scope>
<vibe>...</vibe>";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Distill true context from recent messages and the current task.
///
/// Looks for a `doc_type = "distiller"` document in `context_docs` for the
/// prompt template. Falls back to [`DEFAULT_DISTILLER_PROMPT`] if none found.
///
/// Tag names are extracted from the document's YAML front matter. When no
/// front matter is present, defaults to `["scope", "vibe"]`.
///
/// Returns `None` if the API key is missing, the call fails, or parsing fails —
/// the caller should proceed without context rather than blocking.
pub async fn distill_true_context(
    messages: &[Message],
    task_title: &str,
    task_description: &str,
    context_docs: &[DocumentRow],
) -> Option<TrueContext> {
    let (template, tag_names) = find_distiller_template_with_tags(context_docs);
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

    match parse_dynamic_context(&response_text, &tag_names) {
        Some(ctx) => Some(ctx),
        None => {
            warn!(
                raw = %response_text,
                tags = ?tag_names,
                "true-context distiller: failed to parse any declared tags"
            );
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Template resolution
// ---------------------------------------------------------------------------

/// Find the distiller prompt template and its declared tag names.
///
/// Returns `(prompt_body, tag_names)`. If the document has YAML front matter
/// with a `tags:` field, those tag names are used. Otherwise falls back to
/// `DEFAULT_TAGS`.
fn find_distiller_template_with_tags(docs: &[DocumentRow]) -> (String, Vec<String>) {
    match docs
        .iter()
        .find(|d| d.doc_type.as_deref() == Some(DISTILLER_DOC_TYPE))
    {
        Some(doc) => {
            let tag_names = parse_front_matter(&doc.content)
                .unwrap_or_else(|| DEFAULT_TAGS.iter().map(|s| (*s).to_string()).collect());
            let body = strip_front_matter(&doc.content);
            (body, tag_names)
        }
        None => (
            DEFAULT_DISTILLER_PROMPT.to_string(),
            DEFAULT_TAGS.iter().map(|s| (*s).to_string()).collect(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Front matter parsing
// ---------------------------------------------------------------------------

/// Parse YAML-style front matter to extract tag names.
///
/// Expects the format:
/// ```text
/// ---
/// tags: [scope, vibe, urgency]
/// ---
/// ```
///
/// Returns `None` if no front matter or no `tags:` line found.
fn parse_front_matter(content: &str) -> Option<Vec<String>> {
    if !content.starts_with("---") {
        return None;
    }
    let rest = &content[3..];
    let end = rest.find("---")?;
    let yaml = &rest[..end];

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("tags:") {
            let value = trimmed.strip_prefix("tags:")?.trim();
            let inner = value.trim_start_matches('[').trim_end_matches(']');
            let tags: Vec<String> = inner
                .split(',')
                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if tags.is_empty() {
                return None;
            }
            return Some(tags);
        }
    }
    None
}

/// Strip YAML front matter from document content, returning the body.
fn strip_front_matter(content: &str) -> String {
    if !content.starts_with("---") {
        return content.to_string();
    }
    let rest = &content[3..];
    match rest.find("---") {
        Some(end) => rest[end + 3..].trim_start_matches('\n').to_string(),
        None => content.to_string(),
    }
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
// Dynamic parser
// ---------------------------------------------------------------------------

/// Parse dynamic context tags from LLM output.
///
/// Extracts `<tag_name>value</tag_name>` for each declared tag name.
/// Returns `None` only if no tags were found at all.
fn parse_dynamic_context(text: &str, tag_names: &[String]) -> Option<TrueContext> {
    let mut fields = HashMap::new();
    for tag in tag_names {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        if let Some(value) = extract_between(text, &open, &close) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                fields.insert(tag.clone(), trimmed.to_string());
            }
        }
    }

    if fields.is_empty() {
        None
    } else {
        Some(TrueContext { fields })
    }
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
            summary: None,
            doc_type: Some(doc_type.to_string()),
            ref_tag: None,
            tags: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            workflow_id: None,
            target_length: None,
            is_static: None,
            source_protocol_step_id: None,
        }
    }

    // -- Front matter parsing ------------------------------------------------

    #[test]
    fn parse_front_matter_valid() {
        let content = "---\ntags: [scope, vibe, urgency]\n---\nPrompt body";
        let tags = parse_front_matter(content).unwrap();
        assert_eq!(tags, vec!["scope", "vibe", "urgency"]);
    }

    #[test]
    fn parse_front_matter_quoted_tags() {
        let content = "---\ntags: [\"intent\", 'mood']\n---\nBody";
        let tags = parse_front_matter(content).unwrap();
        assert_eq!(tags, vec!["intent", "mood"]);
    }

    #[test]
    fn parse_front_matter_no_header() {
        assert!(parse_front_matter("Just a prompt").is_none());
    }

    #[test]
    fn parse_front_matter_no_tags_line() {
        let content = "---\nauthor: bot\n---\nBody";
        assert!(parse_front_matter(content).is_none());
    }

    #[test]
    fn parse_front_matter_empty_tags() {
        let content = "---\ntags: []\n---\nBody";
        assert!(parse_front_matter(content).is_none());
    }

    #[test]
    fn strip_front_matter_basic() {
        let content = "---\ntags: [scope]\n---\nPrompt body here";
        assert_eq!(strip_front_matter(content), "Prompt body here");
    }

    #[test]
    fn strip_front_matter_no_header() {
        let content = "Just a prompt body";
        assert_eq!(strip_front_matter(content), "Just a prompt body");
    }

    #[test]
    fn strip_front_matter_unclosed() {
        let content = "---\ntags: [scope]\nNo closing delimiter";
        assert_eq!(strip_front_matter(content), content);
    }

    // -- Template resolution -------------------------------------------------

    #[test]
    fn template_with_front_matter() {
        let doc_content = "---\ntags: [intent, mood, urgency]\n---\nCustom prompt: {messages}";
        let docs = vec![make_doc("distiller", doc_content)];
        let (template, tags) = find_distiller_template_with_tags(&docs);
        assert_eq!(template, "Custom prompt: {messages}");
        assert_eq!(tags, vec!["intent", "mood", "urgency"]);
    }

    #[test]
    fn template_without_front_matter_uses_default_tags() {
        let doc_content = "Custom prompt without front matter: {messages}";
        let docs = vec![make_doc("distiller", doc_content)];
        let (template, tags) = find_distiller_template_with_tags(&docs);
        assert_eq!(template, doc_content);
        assert_eq!(tags, vec!["scope", "vibe"]);
    }

    #[test]
    fn template_fallback_when_no_distiller_doc() {
        let docs = vec![make_doc("architecture", "arch doc")];
        let (template, tags) = find_distiller_template_with_tags(&docs);
        assert_eq!(template, DEFAULT_DISTILLER_PROMPT);
        assert_eq!(tags, vec!["scope", "vibe"]);
    }

    #[test]
    fn template_fallback_empty_docs() {
        let docs: Vec<DocumentRow> = vec![];
        let (_, tags) = find_distiller_template_with_tags(&docs);
        assert_eq!(tags, vec!["scope", "vibe"]);
    }

    // -- Dynamic parsing -----------------------------------------------------

    #[test]
    fn parse_dynamic_all_tags_present() {
        let text = "<scope>Fix the auth bug.</scope>\n<vibe>Frustrated, third ask.</vibe>";
        let tags = vec!["scope".into(), "vibe".into()];
        let ctx = parse_dynamic_context(text, &tags).unwrap();
        assert_eq!(ctx.fields.get("scope").unwrap(), "Fix the auth bug.");
        assert_eq!(ctx.fields.get("vibe").unwrap(), "Frustrated, third ask.");
    }

    #[test]
    fn parse_dynamic_three_tags() {
        let text = "<scope>API endpoint</scope>\n<vibe>Exploratory</vibe>\n<urgency>High</urgency>";
        let tags = vec!["scope".into(), "vibe".into(), "urgency".into()];
        let ctx = parse_dynamic_context(text, &tags).unwrap();
        assert_eq!(ctx.fields.len(), 3);
        assert_eq!(ctx.fields.get("urgency").unwrap(), "High");
    }

    #[test]
    fn parse_dynamic_partial_tags() {
        let text = "<scope>Fix bug.</scope>";
        let tags = vec!["scope".into(), "vibe".into()];
        let ctx = parse_dynamic_context(text, &tags).unwrap();
        assert_eq!(ctx.fields.len(), 1);
        assert!(ctx.fields.contains_key("scope"));
        assert!(!ctx.fields.contains_key("vibe"));
    }

    #[test]
    fn parse_dynamic_no_tags_found() {
        let text = "No tags in this output.";
        let tags = vec!["scope".into(), "vibe".into()];
        assert!(parse_dynamic_context(text, &tags).is_none());
    }

    #[test]
    fn parse_dynamic_empty_tag_value() {
        let text = "<scope></scope>\n<vibe>ok</vibe>";
        let tags = vec!["scope".into(), "vibe".into()];
        let ctx = parse_dynamic_context(text, &tags).unwrap();
        assert_eq!(ctx.fields.len(), 1);
        assert_eq!(ctx.fields.get("vibe").unwrap(), "ok");
    }

    #[test]
    fn parse_dynamic_with_surrounding_text() {
        let text = "Analysis:\n<intent>Build REST API</intent>\nDone.";
        let tags = vec!["intent".into()];
        let ctx = parse_dynamic_context(text, &tags).unwrap();
        assert_eq!(ctx.fields.get("intent").unwrap(), "Build REST API");
    }

    // -- Extract helper ------------------------------------------------------

    #[test]
    fn extract_between_basic() {
        assert_eq!(
            extract_between("a<b>hello</b>c", "<b>", "</b>"),
            Some("hello")
        );
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
