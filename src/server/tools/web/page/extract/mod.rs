//! HTML → readable markdown.
//!
//! ## Why this is all one blocking closure
//!
//! The `fast_html2md` crate exposes itself as `html2md`.
//!
//! `dom_smoothie` is built on html5ever, whose `StrTendril` and `Document` are
//! `Rc`-backed and therefore `!Send`. `ExecutionStrategy::execute_tool` is an
//! `async_trait` method on a `Send + Sync` trait, so the whole tool future
//! must be `Send`. If any tendril-shaped value were alive across an `.await`
//! the future would stop being `Send` and the impl would not compile.
//!
//! So the DOM stage runs inside a single `spawn_blocking`: owned `String`s go
//! in, owned `String`s come out, and nothing else crosses the boundary. It is
//! also genuinely CPU-bound work that should not sit on an async worker.

use dom_smoothie::{Config, Readability, TextMode};

#[cfg(test)]
mod tests;

/// Readable content extracted from a page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extracted {
    pub title: Option<String>,
    pub byline: Option<String>,
    pub site_name: Option<String>,
    pub published: Option<String>,
    /// Main content as markdown.
    pub markdown: String,
    /// How the content was obtained, for the report's `extracted:` field.
    pub method: Method,
}

/// Which path produced the content.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Method {
    /// Readability found a main article.
    #[default]
    Readability,
    /// Readability declined or failed; the whole body was converted instead.
    WholePage,
    /// The parser panicked or its thread was cancelled; nothing was extracted.
    ///
    /// Distinct from the two success paths so a crash is never reported as an
    /// empty article, and so the caller can skip the JS-shell heuristic — which
    /// would otherwise diagnose a crashed parser as "renders with JavaScript".
    Failed,
}

impl Method {
    pub fn label(&self) -> &'static str {
        match self {
            Method::Readability => "article",
            Method::WholePage => "whole page (no article structure found)",
            Method::Failed => "nothing (the extractor failed on this page)",
        }
    }
}

/// Convert HTML to readable markdown on a blocking thread.
///
/// `url` is used to resolve relative links; an unusable one is not fatal.
pub async fn html_to_markdown(html: String, url: String) -> Extracted {
    tokio::task::spawn_blocking(move || extract_blocking(&html, &url))
        .await
        // A panic inside the parser must degrade to empty content, not take
        // the request down with it. It is reported as `Failed` rather than as
        // the default (`Readability`), so the empty result reads as a failure
        // instead of as an article that genuinely had no text.
        .unwrap_or(Extracted {
            method: Method::Failed,
            ..Default::default()
        })
}

/// The blocking body. Nothing `!Send` escapes this function.
fn extract_blocking(html: &str, url: &str) -> Extracted {
    let cfg = Config {
        text_mode: TextMode::Markdown,
        ..Default::default()
    };

    if let Ok(mut r) = Readability::new(html, Some(url), Some(cfg)) {
        if r.is_probably_readable() {
            if let Ok(article) = r.parse() {
                let markdown = article.text_content.to_string();
                if !markdown.trim().is_empty() {
                    return Extracted {
                        title: non_empty(article.title),
                        byline: article.byline.and_then(non_empty),
                        site_name: article.site_name.and_then(non_empty),
                        published: article.published_time.and_then(non_empty),
                        markdown,
                        method: Method::Readability,
                    };
                }
            }
        }
    }

    // Readability declined (a landing page, a listing, an app shell) or found
    // nothing. Converting the whole body is worse but honest, and the report
    // says which path was taken.
    Extracted {
        title: title_from_html(html),
        markdown: html2md::rewrite_html(html, false).trim().to_string(),
        method: Method::WholePage,
        ..Default::default()
    }
}

/// Pull `<title>` out of raw HTML for the fallback path.
fn title_from_html(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let open_end = lower[start..].find('>')? + start + 1;
    let end = lower[open_end..].find("</title>")? + open_end;
    non_empty(html[open_end..end].trim().to_string())
}

fn non_empty(s: String) -> Option<String> {
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Whether the page looks like a JavaScript shell rather than content.
///
/// A single-page app returns a near-empty body with a script bundle. Saying so
/// is far more useful to an agent than handing it a nav bar and a spinner.
pub fn looks_like_js_shell(extracted: &Extracted, html_len: usize) -> bool {
    const MIN_CONTENT_CHARS: usize = 200;
    const SUSPICIOUS_HTML_CHARS: usize = 1_000;

    extracted.markdown.trim().chars().count() < MIN_CONTENT_CHARS
        && html_len > SUSPICIOUS_HTML_CHARS
}
