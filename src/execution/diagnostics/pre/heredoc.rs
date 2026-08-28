//! Detects a `run_command` whose input was cut off mid-heredoc.
//!
//! The heredoc body travels inside the `tool_use` JSON, so it is bounded by the
//! model's `max_tokens`. When it is cut, the shell still runs the fragment and
//! writes a truncated file that reports success. In run dd27d008 the
//! design-spec agent burned three rounds and ~2m40s rediscovering this, blamed
//! the shell, and left an 816-line deliverable corrupt on disk for two minutes.

use super::super::envelope::{Diagnostic, DiagnosticCategory, Severity};
use super::{find_unquoted, PreCheck};

/// Names of heredoc delimiters opened but never closed, in the order opened.
///
/// Pure and side-effect free — safe to call from any tool path, including the
/// container dispatcher, which has no diagnostics engine.
pub fn unterminated_heredocs(command: &str) -> Vec<String> {
    let chars: Vec<char> = command.chars().collect();
    let mut open: Vec<(String, bool)> = Vec::new();

    for &pos in &find_unquoted(command, "<<") {
        let mut i = pos + 2;

        // `<<<` is a here-string: no terminator, no body.
        if chars.get(i) == Some(&'<') {
            continue;
        }

        // `<<-` strips leading tabs from the terminator line.
        let dash = chars.get(i) == Some(&'-');
        if dash {
            i += 1;
        }

        while chars.get(i).is_some_and(|c| *c == ' ' || *c == '\t') {
            i += 1;
        }

        let delimiter = match chars.get(i) {
            Some('\'') | Some('"') => {
                let quote = chars[i];
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                chars[start..i].iter().collect::<String>()
            }
            Some(_) => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
                {
                    i += 1;
                }
                chars[start..i].iter().collect::<String>()
            }
            None => String::new(),
        };

        if !delimiter.is_empty() {
            open.push((delimiter, dash));
        }
    }

    if open.is_empty() {
        return Vec::new();
    }

    // Walk the body lines in order, closing delimiters as their terminator
    // appears. A terminator line is the delimiter alone, tabs stripped for
    // `<<-`.
    let mut remaining = open;

    for line in command.lines().skip(1) {
        let Some((delimiter, dash)) = remaining.first() else {
            break;
        };
        let candidate = if *dash {
            line.trim_start_matches('\t')
        } else {
            line
        };
        if candidate.trim_end() == delimiter {
            remaining.remove(0);
        }
    }

    remaining.into_iter().map(|(d, _)| d).collect()
}

/// Blocks a command whose heredoc was cut off before its terminator.
pub struct HeredocCheck;

impl PreCheck for HeredocCheck {
    fn check(&self, command: &str) -> Option<Diagnostic> {
        let open = unterminated_heredocs(command);
        if open.is_empty() {
            return None;
        }
        Some(Diagnostic {
            // Error + Truncation is the abort signal `DiagnosticsEngine::execute`
            // reads. Nothing else emits that combination.
            severity: Severity::Error,
            category: DiagnosticCategory::Truncation,
            message: format!(
                "Your command was cut off before its heredoc closed ({} never appears on \
                 a line of its own). It was NOT run — running it would have written a \
                 truncated file that looked like a success.",
                open.join(", ")
            ),
            suggestion: Some(
                "The heredoc body is part of your response, so it is bounded by your output \
                 limit. Use write_file for the first section, then edit_file with an empty \
                 old_string to append the rest."
                    .to_string(),
            ),
        })
    }
}
