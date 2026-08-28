//! The `read_webpage` tool: fetch a URL and return readable text.

use std::time::Duration;

use futures::StreamExt;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use url::Url;

use super::brief;
use super::format::{truncate_chars, Envelope, RESULT_SUCCESS, RESULT_WARNING};
use crate::net::egress;
use crate::server::tools::shared::error_json;

pub mod extract;
pub mod guard;

#[cfg(test)]
mod tests;

/// Budget for a single hop: connect, headers, and body.
///
/// `reqwest`'s `timeout` is per request, and the redirect chain is followed
/// manually, so this alone would let `MAX_HOPS` slow hops stack up. The whole
/// call is separately bounded by [`TOTAL_TIMEOUT_SECS`].
const TIMEOUT_SECS: u64 = 45;
/// Budget for the whole `read_webpage` call, across every redirect hop.
///
/// Without this a chain of eight hosts that each stall just under the per-hop
/// limit holds a worker for six minutes, and the engine puts no timeout of its
/// own around a tool call.
const TOTAL_TIMEOUT_SECS: u64 = 60;
/// Redirect hops followed before giving up.
const MAX_HOPS: usize = 8;
/// Hard cap on bytes read from the network, enforced while streaming so a
/// dishonest or absent Content-Length cannot make us buffer a huge body.
const MAX_BYTES: usize = 8 * 1024 * 1024;
/// Characters of content returned in one call.
const MAX_CONTENT_CHARS: usize = 40_000;

/// Execute a `read_webpage` call.
pub async fn execute(input: &Value) -> Value {
    let raw = match input.get("url").and_then(Value::as_str) {
        Some(u) if !u.trim().is_empty() => u.trim(),
        _ => return error_json("Missing required parameter: url"),
    };
    // A malformed offset is refused rather than coerced to 0: silently
    // returning page 1 alongside a "call again with offset=N" note is a loop
    // the engine's breaker cannot see, because every call succeeds.
    let offset = match input.get("offset") {
        None | Some(Value::Null) => 0usize,
        Some(v) => match v.as_u64() {
            Some(n) => n as usize,
            None => {
                return error_json(format!(
                    "Invalid offset {v}: expected a whole number of characters, 0 or greater"
                ))
            }
        },
    };

    let start = match guard::validate(raw) {
        Ok(u) => u,
        Err(e) => return error_json(format!("Cannot fetch '{raw}': {e}")),
    };

    let client = match egress::client_no_redirect(Duration::from_secs(TIMEOUT_SECS)) {
        Ok(c) => c,
        Err(e) => return error_json(e.to_string()),
    };

    let fetched = tokio::time::timeout(
        Duration::from_secs(TOTAL_TIMEOUT_SECS),
        fetch(&client, start),
    )
    .await;

    match fetched {
        Ok(Ok(page)) => render(&page, offset).await,
        Ok(Err(e)) => error_json(e.to_string()),
        Err(_) => error_json(format!(
            "Could not fetch the page: the request did not complete within {TOTAL_TIMEOUT_SECS}s"
        )),
    }
}

/// A fetched document.
pub(crate) struct Page {
    pub(crate) final_url: Url,
    pub(crate) content_type: String,
    pub(crate) body: String,
    pub(crate) truncated_download: bool,
}

/// Why a fetch could not produce content.
#[derive(Debug)]
pub(crate) enum FetchError {
    Rejected(String),
    Status(StatusCode, String),
    TooManyHops,
    Unsupported(String),
    Transport(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Rejected(m) => write!(f, "{m}"),
            FetchError::Status(code, url) => match *code {
                StatusCode::NOT_FOUND => write!(f, "Page not found (404): {url}"),
                StatusCode::FORBIDDEN => write!(
                    f,
                    "Access denied (403) by {url} — the site blocks automated readers. \
                     Try a different source."
                ),
                StatusCode::TOO_MANY_REQUESTS => {
                    write!(f, "Rate limited (429) by {url} — wait before retrying")
                }
                StatusCode::UNAUTHORIZED => {
                    write!(f, "Login required (401): {url}")
                }
                c if c.is_server_error() => {
                    write!(f, "The site returned a server error ({c}): {url}")
                }
                c => write!(f, "Fetch failed with HTTP {c}: {url}"),
            },
            FetchError::TooManyHops => {
                write!(f, "Too many redirects — the URL does not settle on a page")
            }
            FetchError::Unsupported(t) => write!(
                f,
                "Cannot read '{t}' content as text. read_webpage handles HTML, \
                 plain text, JSON and XML."
            ),
            FetchError::Transport(m) => write!(f, "Could not fetch the page: {m}"),
        }
    }
}

/// Fetch a URL, following redirects manually so each hop is re-validated.
pub(crate) async fn fetch(client: &Client, start: Url) -> Result<Page, FetchError> {
    let mut url = start;
    // Only when this process resolves names itself; under the proxy the
    // lookup happens on the far side and checking here would prove nothing.
    let check_dns = egress::resolves_locally();

    for _ in 0..MAX_HOPS {
        if check_dns {
            guard::validate_addresses(&url)
                .await
                .map_err(|e| FetchError::Rejected(format!("Cannot fetch '{url}': {e}")))?;
        }

        let response = client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| FetchError::Transport(brief(&e)))?;

        let status = response.status();

        if status.is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok());
            url = next_hop(&url, location)?;
            continue;
        }

        if !status.is_success() {
            return Err(FetchError::Status(status, url.to_string()));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let kind = classify(&content_type);
        if kind == Kind::Unsupported {
            return Err(FetchError::Unsupported(
                content_type
                    .split(';')
                    .next()
                    .unwrap_or("unknown")
                    .trim()
                    .to_string(),
            ));
        }

        let (bytes, truncated) = read_capped(response).await?;
        let body = decode(&bytes, &content_type);

        return Ok(Page {
            final_url: url,
            content_type,
            body,
            truncated_download: truncated,
        });
    }

    Err(FetchError::TooManyHops)
}

/// Resolve and validate the next hop of a redirect chain.
///
/// Split out from [`fetch`] because it is the security-relevant half of the
/// loop and the only half that can be tested without a network: a mock server
/// binds to loopback, which this function is required to refuse.
fn next_hop(current: &Url, location: Option<&str>) -> Result<Url, FetchError> {
    let location = location
        .ok_or_else(|| FetchError::Rejected("Redirect without a destination".to_string()))?;

    // Relative Location headers are legal and common.
    let next = current
        .join(location)
        .map_err(|_| FetchError::Rejected(format!("Invalid redirect target: {location}")))?;

    guard::validate_hop(&next)
        .map_err(|e| FetchError::Rejected(format!("Refused redirect to '{next}': {e}")))?;

    Ok(next)
}

/// Read the body with a cap enforced *while* streaming.
///
/// Content-Length is not trusted: it is absent on chunked responses, reported
/// as `None` once reqwest transparently decompresses, and can simply lie.
async fn read_capped(response: reqwest::Response) -> Result<(Vec<u8>, bool), FetchError> {
    let mut buf: Vec<u8> = Vec::with_capacity(64 * 1024);
    let mut stream = response.bytes_stream();
    let mut truncated = false;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| FetchError::Transport(brief(&e)))?;
        let remaining = MAX_BYTES.saturating_sub(buf.len());
        if chunk.len() > remaining {
            buf.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        buf.extend_from_slice(&chunk);
    }
    Ok((buf, truncated))
}

/// Content kinds this tool can turn into text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Html,
    Text,
    Unsupported,
}

/// Classify a Content-Type header.
///
/// A missing or empty type is treated as HTML: servers omit it more often than
/// they serve binaries without one, and the extractor degrades safely on text.
pub(crate) fn classify(content_type: &str) -> Kind {
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    if mime.is_empty() {
        return Kind::Html;
    }
    if mime == "text/html" || mime == "application/xhtml+xml" {
        return Kind::Html;
    }
    if mime.starts_with("text/")
        || mime == "application/json"
        || mime == "application/xml"
        || mime.ends_with("+json")
        || mime.ends_with("+xml")
    {
        return Kind::Text;
    }
    Kind::Unsupported
}

/// Decode bytes to a string, honouring the declared charset.
///
/// Falls back to UTF-8 with replacement rather than failing: a page with a
/// mis-declared encoding is still worth reading.
pub(crate) fn decode(bytes: &[u8], content_type: &str) -> String {
    // HTTP parameter names are case-insensitive; `Charset=` is legal.
    let label = content_type.split(';').find_map(|p| {
        let p = p.trim();
        let (name, value) = p.split_once('=')?;
        name.trim()
            .eq_ignore_ascii_case("charset")
            .then(|| value.trim().trim_matches(['"', '\'']))
    });

    let encoding = label
        .and_then(|l| encoding_rs::Encoding::for_label(l.as_bytes()))
        // Sniffing is an HTML rule. Applying it to JSON or plain text
        // mis-decodes any document that merely mentions `charset=` in its
        // first 2 KiB.
        .or_else(|| match classify(content_type) {
            Kind::Html => sniff_meta_charset(bytes),
            _ => None,
        })
        .unwrap_or(encoding_rs::UTF_8);

    let (text, _, _) = encoding.decode(bytes);
    text.into_owned()
}

/// Look for `<meta charset=...>` in the first part of the document.
fn sniff_meta_charset(bytes: &[u8]) -> Option<&'static encoding_rs::Encoding> {
    let head = &bytes[..bytes.len().min(2048)];
    let text = String::from_utf8_lossy(head).to_ascii_lowercase();
    let idx = text.find("charset=")? + "charset=".len();
    // The value may be quoted: charset="utf-8" or charset='utf-8'.
    let rest = text[idx..].trim_start_matches(['"', '\'']);
    let end = rest
        .find(|c: char| {
            c == '"' || c == '\'' || c == '>' || c == ';' || c == '/' || c.is_whitespace()
        })
        .unwrap_or(rest.len());
    encoding_rs::Encoding::for_label(rest[..end].trim().as_bytes())
}

/// Render the report the model reads.
pub(crate) async fn render(page: &Page, offset: usize) -> Value {
    let kind = classify(&page.content_type);

    let (extracted, is_html) = match kind {
        Kind::Html => (
            extract::html_to_markdown(page.body.clone(), page.final_url.to_string()).await,
            true,
        ),
        _ => (
            extract::Extracted {
                markdown: page.body.trim().to_string(),
                ..Default::default()
            },
            false,
        ),
    };

    let failed = extracted.method == extract::Method::Failed;
    // A crashed parser also returns almost no text, so the JS-shell heuristic
    // would fire and confidently misdiagnose it as a page that needs a browser.
    let js_shell = is_html && !failed && extract::looks_like_js_shell(&extracted, page.body.len());

    let total: usize = extracted.markdown.chars().count();
    let past_end = offset > 0 && offset >= total;

    let mut env = Envelope::new(
        if js_shell || failed || past_end || page.truncated_download {
            RESULT_WARNING
        } else {
            RESULT_SUCCESS
        },
    );

    env.field("url", page.final_url.as_str());
    env.field_opt("title", extracted.title.as_deref());
    env.field_opt("site", extracted.site_name.as_deref());
    env.field_opt("byline", extracted.byline.as_deref());
    env.field_opt("published", extracted.published.as_deref());
    if is_html {
        env.field("extracted", extracted.method.label());
    }

    let body: String = extracted.markdown.chars().skip(offset).collect();
    let shown = truncate_chars(&body, MAX_CONTENT_CHARS);
    let start = offset.min(total);
    // Clamped against `total`, not derived from `offset`: an offset past the
    // end would otherwise report `chars 5-9999 of 5`.
    let end = (start + shown.text.chars().count()).min(total);

    env.field("content", format!("chars {start}-{end} of {total}"));

    if past_end {
        env.note(&format!(
            "The offset {offset} is past the end of this page, which is {total} \
             characters long. Nothing follows."
        ));
    }
    if failed {
        env.note(
            "The extractor could not process this page, so no text was \
             recovered. This is a failure on our side, not an empty page — \
             the content below, if any, is not the article.",
        );
    }
    if js_shell {
        env.note(
            "This page returned almost no readable content for its size, which \
             usually means it renders in the browser with JavaScript. What \
             follows may be a shell rather than the article.",
        );
    }
    if page.truncated_download {
        env.note("The download hit its size limit; the page was cut off mid-transfer.");
    }

    // The content is written by whoever controls the URL. Fence it and say so,
    // so instructions embedded in a page read as quoted data rather than as
    // something addressed to the agent.
    env.note("--- begin untrusted page content ---");
    env.block(&shown.text);
    env.note("--- end untrusted page content ---");

    if shown.truncated {
        env.note(&format!(
            "Truncated. To continue, call read_webpage again with offset={end}.",
        ));
    }

    Value::String(env.finish())
}
