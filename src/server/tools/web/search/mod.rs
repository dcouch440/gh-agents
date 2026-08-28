//! The `brave_search` tool: web search via the Brave Web Search API.
//!
//! Returns a labeled plain-text report rather than JSON. The engine forwards a
//! `Value::String` to the model verbatim, so what is rendered here is exactly
//! what the agent reads.

use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use super::format::{squeeze_ws, strip_highlight_tags, truncate_chars, Envelope, RESULT_SUCCESS};
use crate::net::egress;
use crate::server::tools::shared::error_json;

#[cfg(test)]
mod tests;

/// Results requested from Brave. The API caps `count` at 20; ten is enough to
/// choose what to read without burying the agent in near-duplicates.
const RESULT_COUNT: u8 = 10;
/// Characters kept from each result description.
const DESCRIPTION_CHARS: usize = 280;
/// Characters kept from each extra snippet.
const SNIPPET_CHARS: usize = 200;
/// Extra snippets rendered per result. Brave returns up to five; two is the
/// point where they stop adding signal per token spent.
const SNIPPETS_PER_RESULT: usize = 2;
/// Whole-request timeout.
const TIMEOUT_SECS: u64 = 20;
/// Below this remaining monthly quota the report carries a warning, so the
/// agent (and whoever reads the transcript) learns before the tool goes dead.
const LOW_QUOTA_THRESHOLD: u64 = 100;

/// Execute a `brave_search` call.
pub async fn execute(input: &Value) -> Value {
    let query = match input.get("query").and_then(Value::as_str) {
        Some(q) if !q.trim().is_empty() => q.trim(),
        _ => return error_json("Missing required parameter: query"),
    };

    let api_key = match std::env::var(crate::constants::ENV_BRAVE_SEARCH_API_KEY) {
        Ok(k) if !k.is_empty() => k,
        _ => {
            return error_json(format!(
                "Web search is not configured: {} is not set",
                crate::constants::ENV_BRAVE_SEARCH_API_KEY
            ))
        }
    };

    let client = match egress::client(Duration::from_secs(TIMEOUT_SECS)) {
        Ok(c) => c,
        // Fail closed: the query itself is sensitive, so a search that cannot
        // use the tunnel must not be retried without it.
        Err(e) => return error_json(e.to_string()),
    };

    let freshness = input.get("freshness").and_then(Value::as_str);
    let mut params: Vec<(&str, String)> = vec![
        ("q", query.to_string()),
        ("count", RESULT_COUNT.to_string()),
        ("extra_snippets", "true".to_string()),
    ];
    if let Some(f) = freshness.filter(|f| is_valid_freshness(f)) {
        params.push(("freshness", f.to_string()));
    }

    let response = match client
        .get(crate::constants::BRAVE_SEARCH_ENDPOINT)
        .header("Accept", "application/json")
        .header("X-Subscription-Token", &api_key)
        .query(&params)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return error_json(format!("Web search request failed: {}", brief(&e))),
    };

    let status = response.status();
    let quota = remaining_month_quota(response.headers());

    if !status.is_success() {
        return error_json(match status.as_u16() {
            401 | 403 => "Web search rejected the API key".to_string(),
            429 => "Web search rate limit reached; wait before searching again".to_string(),
            422 => format!("Web search rejected the query: {query}"),
            code => format!("Web search failed with HTTP {code}"),
        });
    }

    let body: BraveResponse = match response.json().await {
        Ok(b) => b,
        Err(e) => return error_json(format!("Could not read web search results: {}", brief(&e))),
    };

    Value::String(render(query, &body, quota))
}

/// Render the report the model reads.
fn render(query: &str, body: &BraveResponse, quota: Option<u64>) -> String {
    let results = body
        .web
        .as_ref()
        .map(|w| w.results.as_slice())
        .unwrap_or(&[]);

    let mut env = Envelope::new(RESULT_SUCCESS);
    env.field("query", query);
    env.field("results", results.len().to_string());

    if results.is_empty() {
        // Not an error: a search that legitimately finds nothing is a fact the
        // agent needs, and returning an error would trip the failure breaker.
        env.note("No results. Try different or broader terms.");
        return env.finish();
    }

    if body
        .query
        .as_ref()
        .and_then(|q| q.bad_results)
        .unwrap_or(false)
    {
        env.field("quality", "low — Brave flagged these results as weak");
    }

    if let Some(remaining) = quota {
        if remaining <= LOW_QUOTA_THRESHOLD {
            env.field("quota", format!("{remaining} searches left this month"));
        }
    }

    env.section("results");
    for (i, r) in results.iter().enumerate() {
        let title = squeeze_ws(&strip_highlight_tags(&r.title));
        env.line(&format!("[{}] {}", i + 1, title));
        env.line(&format!("    {}", r.url));

        let site = r.meta_url.as_ref().and_then(|m| m.hostname.as_deref());
        match (site, r.age.as_deref()) {
            (Some(s), Some(a)) => env.line(&format!("    {s} · {a}")),
            (Some(s), None) => env.line(&format!("    {s}")),
            (None, Some(a)) => env.line(&format!("    {a}")),
            (None, None) => &mut env,
        };

        if let Some(d) = &r.description {
            let d = truncate_chars(&squeeze_ws(&strip_highlight_tags(d)), DESCRIPTION_CHARS);
            env.line(&format!("    {}", d.text));
        }
        for snip in r.extra_snippets.iter().flatten().take(SNIPPETS_PER_RESULT) {
            let s = truncate_chars(&squeeze_ws(&strip_highlight_tags(snip)), SNIPPET_CHARS);
            env.line(&format!("    - {}", s.text));
        }
        if i + 1 < results.len() {
            env.line("");
        }
    }

    env.note(
        "Results are search snippets, not sources. Use read_webpage on a URL \
         above before relying on what it says.",
    );
    env.finish()
}

/// Whether a freshness value is one Brave accepts.
///
/// Validated rather than passed through: an invalid value makes Brave reject
/// the whole request, turning a slightly-wrong parameter into a failed search.
fn is_valid_freshness(v: &str) -> bool {
    matches!(v, "pd" | "pw" | "pm" | "py")
}

/// Remaining monthly quota from `x-ratelimit-remaining`.
///
/// Brave reports two comma-separated windows, per-second then per-month; the
/// month is the one worth surfacing.
fn remaining_month_quota(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get("x-ratelimit-remaining")?
        .to_str()
        .ok()?
        .split(',')
        .nth(1)?
        .trim()
        .parse()
        .ok()
}

/// A short, non-leaking description of a transport error.
fn brief(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "timed out".to_string()
    } else if e.is_connect() {
        "could not connect".to_string()
    } else {
        "network error".to_string()
    }
}

// ── Wire types ──────────────────────────────────────────────────────────────
//
// Every section is optional. A live search for an ordinary query returns only
// `query`, `web`, `mixed`, `type` and `videos` — a required field here would
// fail to deserialize on the common case.

#[derive(Debug, Deserialize)]
pub(crate) struct BraveResponse {
    #[serde(default)]
    pub(crate) query: Option<BraveQuery>,
    #[serde(default)]
    pub(crate) web: Option<BraveWeb>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BraveQuery {
    #[serde(default)]
    pub(crate) bad_results: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BraveWeb {
    #[serde(default)]
    pub(crate) results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BraveResult {
    #[serde(default)]
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) age: Option<String>,
    #[serde(default)]
    pub(crate) meta_url: Option<BraveMetaUrl>,
    #[serde(default)]
    pub(crate) extra_snippets: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BraveMetaUrl {
    #[serde(default)]
    pub(crate) hostname: Option<String>,
}
