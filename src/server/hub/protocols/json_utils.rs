//! Shared JSON extraction utilities for protocol and strategy outputs.
//!
//! LLMs often wrap JSON responses in markdown code fences or surrounding prose.
//! These utilities strip that wrapping to extract clean JSON content.

/// Extract JSON content from an LLM response, stripping markdown code fences if present.
///
/// Tries in order:
/// 1. Raw JSON parse (content is already valid JSON)
/// 2. `` ```json `` fenced code block
/// 3. Bare `` ``` `` fenced code block
/// 4. First `{…}` or `[…]` extraction
///
/// Returns the extracted string. Callers can parse to `serde_json::Value` as needed.
pub fn extract_json_from_llm_response(content: &str) -> String {
    let trimmed = content.trim();

    // Try raw parse first
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return trimmed.to_string();
    }

    // Try ```json fence
    if let Some(start) = trimmed.find("```json") {
        if let Some(end) = trimmed[start + 7..].find("```") {
            return trimmed[start + 7..start + 7 + end].trim().to_string();
        }
    }

    // Try bare ``` fence
    if let Some(start) = trimmed.find("```") {
        if let Some(end) = trimmed[start + 3..].find("```") {
            return trimmed[start + 3..start + 3 + end].trim().to_string();
        }
    }

    // Try extracting { ... }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return trimmed[start..=end].to_string();
        }
    }

    trimmed.to_string()
}

/// Parse structured JSON output from an LLM response.
///
/// Extracts JSON using [`extract_json_from_llm_response`], then parses it
/// to a `serde_json::Value`. Returns `None` if the content contains no
/// valid JSON object or array.
pub fn parse_structured_output(content: &str) -> Option<serde_json::Value> {
    let extracted = extract_json_from_llm_response(content);
    let trimmed = extracted.trim();

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if v.is_object() || v.is_array() {
            return Some(v);
        }
    }

    None
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_json_from_llm_response ──────────────────────────────────

    #[test]
    fn extract_raw_json() {
        let raw = r#"{"documents": [{"name": "API Ref"}]}"#;
        let result = extract_json_from_llm_response(raw);
        assert_eq!(result, raw);
    }

    #[test]
    fn extract_json_fence() {
        let fenced = "```json\n{\"documents\": [{\"name\": \"API Ref\"}]}\n```";
        let result = extract_json_from_llm_response(fenced);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["documents"][0]["name"], "API Ref");
    }

    #[test]
    fn extract_bare_fence() {
        let fenced = "```\n{\"documents\": [{\"name\": \"Guide\"}]}\n```";
        let result = extract_json_from_llm_response(fenced);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["documents"][0]["name"], "Guide");
    }

    #[test]
    fn extract_surrounding_text() {
        let messy = "Here is the plan:\n{\"documents\": [{\"name\": \"Overview\"}]}\nThat's it.";
        let result = extract_json_from_llm_response(messy);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["documents"][0]["name"], "Overview");
    }

    #[test]
    fn extract_json_fence_with_preamble() {
        let content = "I'll create a documentation plan:\n\n```json\n{\"documents\": [{\"name\": \"Architecture\", \"capabilities\": [\"web_search\"]}]}\n```\n\nThis plan covers the main topics.";
        let result = extract_json_from_llm_response(content);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["documents"][0]["name"], "Architecture");
    }

    // ── parse_structured_output ─────────────────────────────────────────

    #[test]
    fn parse_direct_json() {
        let result = parse_structured_output(r#"{"key": "value"}"#);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["key"], "value");
    }

    #[test]
    fn parse_code_fence() {
        let input = "Here is the result:\n```json\n{\"key\": \"value\"}\n```";
        let result = parse_structured_output(input);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["key"], "value");
    }

    #[test]
    fn parse_embedded_json() {
        let input = "The answer is {\"key\": \"value\"} as shown.";
        let result = parse_structured_output(input);
        assert!(result.is_some());
    }

    #[test]
    fn parse_plain_text_returns_none() {
        let result = parse_structured_output("Just plain text, no JSON here.");
        assert!(result.is_none());
    }

    #[test]
    fn parse_array() {
        let result = parse_structured_output(r#"[{"a": 1}, {"a": 2}]"#);
        assert!(result.is_some());
        assert!(result.unwrap().is_array());
    }
}
