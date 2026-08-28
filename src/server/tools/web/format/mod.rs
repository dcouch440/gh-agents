//! Shared plain-text envelope for web tool results.
//!
//! Web tools return their success payload as a `serde_json::Value::String`,
//! which the execution engine forwards to the model verbatim (see
//! `crate::server::hub::execution::engine`). That makes the rendering here the
//! literal text the model reads, so it follows the house voice already
//! established by `crate::execution::diagnostics::envelope::CommandEnvelope`:
//! a `result:` line, lowercase `key: value` fields, blank-line-separated
//! sections, and two-space indented bodies.

use std::fmt::Write as _;

#[cfg(test)]
mod tests;

/// Result labels, matching `diagnostics::envelope::Severity::label`.
pub const RESULT_SUCCESS: &str = "success";
pub const RESULT_WARNING: &str = "warning";

/// Indent applied to every section body line.
const INDENT: &str = "  ";

/// Builder for a labeled plain-text tool result.
///
/// # Examples
///
/// ```
/// use nexor::server::tools::web::format::{Envelope, RESULT_SUCCESS};
///
/// let mut env = Envelope::new(RESULT_SUCCESS);
/// env.field("query", "rust async");
/// env.field("results", "2");
/// let out = env.finish();
/// assert!(out.starts_with("result: success\n"));
/// assert!(out.contains("query: rust async"));
/// ```
pub struct Envelope {
    out: String,
}

impl Envelope {
    /// Start an envelope with the given `result:` label.
    pub fn new(result: &str) -> Self {
        let mut out = String::with_capacity(512);
        let _ = writeln!(out, "result: {}", result);
        Self { out }
    }

    /// Append a `key: value` field line.
    ///
    /// The value is whitespace-collapsed. Field values carry page-controlled
    /// text — `<title>`, `og:site_name`, bylines, result URLs — and the header
    /// region sits *above* the untrusted-content fence and is not indented. A
    /// newline in a value would let a page emit its own unindented lines there,
    /// forging envelope framing that the fence is supposed to contain.
    pub fn field(&mut self, key: &str, value: impl AsRef<str>) -> &mut Self {
        let _ = writeln!(self.out, "{}: {}", key, squeeze_ws(value.as_ref()));
        self
    }

    /// Append a `key: value` line only when the value is present and non-empty.
    pub fn field_opt(&mut self, key: &str, value: Option<impl AsRef<str>>) -> &mut Self {
        if let Some(v) = value {
            // Emptiness is judged after collapsing: a metadata value that is
            // only whitespace would otherwise emit a bare `key:` line.
            let v = squeeze_ws(v.as_ref());
            if !v.is_empty() {
                self.field(key, v);
            }
        }
        self
    }

    /// Open a titled section: a blank line, then `heading:`.
    pub fn section(&mut self, heading: &str) -> &mut Self {
        let _ = writeln!(self.out, "\n{}:", heading);
        self
    }

    /// Append one indented body line.
    ///
    /// Embedded newlines are flattened to spaces. Only the first line of the
    /// argument would receive [`INDENT`], so a value containing a newline —
    /// a scraped result URL, a date field — could otherwise emit unindented
    /// text that mimics the envelope's own framing. [`block`] has already split
    /// on newlines before it gets here, so this never disturbs a body.
    ///
    /// [`block`]: Self::block
    pub fn line(&mut self, line: &str) -> &mut Self {
        if line.contains(['\n', '\r']) {
            let flat: String = line
                .chars()
                .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
                .collect();
            let _ = writeln!(self.out, "{}{}", INDENT, flat);
        } else {
            let _ = writeln!(self.out, "{}{}", INDENT, line);
        }
        self
    }

    /// Append a multi-line body, indenting every line.
    pub fn block(&mut self, body: &str) -> &mut Self {
        for line in body.lines() {
            self.line(line);
        }
        self
    }

    /// Append a standalone paragraph with a leading blank line, unindented.
    pub fn note(&mut self, message: &str) -> &mut Self {
        let _ = writeln!(self.out, "\n{}", message);
        self
    }

    /// Finish the envelope, trimming trailing whitespace.
    pub fn finish(self) -> String {
        let trimmed = self.out.trim_end();
        let mut s = String::with_capacity(trimmed.len() + 1);
        s.push_str(trimmed);
        s.push('\n');
        s
    }
}

/// Outcome of a character-safe truncation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Truncated {
    /// The (possibly shortened) text.
    pub text: String,
    /// Whether anything was removed.
    pub truncated: bool,
    /// Character count of the original input.
    pub original_chars: usize,
}

impl Truncated {
    /// The text, with an ellipsis when it was cut.
    ///
    /// Without the marker a description clipped mid-sentence reads as a
    /// complete one that simply stops, and the model quotes the fragment as
    /// the page's whole claim.
    pub fn with_ellipsis(&self) -> String {
        if self.truncated {
            format!("{}…", self.text)
        } else {
            self.text.clone()
        }
    }

    /// A short human summary suitable for a `key: value` field.
    pub fn summary(&self) -> String {
        if self.truncated {
            format!(
                "{} of {} chars (truncated)",
                self.text.chars().count(),
                self.original_chars
            )
        } else {
            format!("{} chars", self.original_chars)
        }
    }
}

/// Truncate on a character boundary, never a byte one.
///
/// Byte slicing a `&str` panics on multi-byte input; scraped pages are full of
/// it, so this is the only truncation the web tools use.
///
/// # Examples
///
/// ```
/// use nexor::server::tools::web::format::truncate_chars;
///
/// let t = truncate_chars("héllo wörld", 5);
/// assert_eq!(t.text, "héllo");
/// assert!(t.truncated);
/// assert_eq!(t.original_chars, 11);
///
/// assert!(!truncate_chars("short", 99).truncated);
/// ```
pub fn truncate_chars(s: &str, max_chars: usize) -> Truncated {
    let original_chars = s.chars().count();
    if original_chars <= max_chars {
        return Truncated {
            text: s.to_string(),
            truncated: false,
            original_chars,
        };
    }
    Truncated {
        text: s.chars().take(max_chars).collect(),
        truncated: true,
        original_chars,
    }
}

/// Collapse runs of whitespace into single spaces and trim.
///
/// Search descriptions arrive with embedded newlines and non-breaking spaces
/// that would otherwise break the indented layout.
pub fn squeeze_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Strip Brave's `<strong>` highlight markup from a snippet.
///
/// Brave marks query-term matches inline. The tags carry no meaning once the
/// text reaches the model and only cost tokens.
pub fn strip_highlight_tags(s: &str) -> String {
    s.replace("<strong>", "").replace("</strong>", "")
}
