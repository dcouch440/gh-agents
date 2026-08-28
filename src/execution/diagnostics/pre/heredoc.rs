//! Detects a `run_command` whose input was cut off mid-heredoc.
//!
//! The heredoc body travels inside the `tool_use` JSON, so it is bounded by the
//! model's `max_tokens`. When it is cut, the shell still runs the fragment and
//! writes a truncated file that reports success. In run dd27d008 the
//! design-spec agent burned three rounds and ~2m40s rediscovering this, blamed
//! the shell, and left an 816-line deliverable corrupt on disk for two minutes.

use super::super::envelope::{Diagnostic, DiagnosticCategory, Severity};
use super::{find_unquoted_from, PreCheck};

/// Heredoc openers on a single command line, in the order they appear.
///
/// `in_single`/`in_double` carry the quote state in from the previous command
/// line and out to the next, so a `<<` inside a multi-line quoted string stays
/// a shift operator.
fn openers_in_line(line: &str, in_single: &mut bool, in_double: &mut bool) -> Vec<(String, bool)> {
    let chars: Vec<char> = line.chars().collect();
    let mut found = Vec::new();

    for pos in find_unquoted_from(line, "<<", in_single, in_double) {
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
            found.push((delimiter, dash));
        }
    }

    found
}

/// Whether `line` is the terminator closing `delimiter`.
fn closes(line: &str, delimiter: &str, dash: bool) -> bool {
    let candidate = if dash {
        line.trim_start_matches('\t')
    } else {
        line
    };
    candidate.trim_end() == delimiter
}

/// Names of heredoc delimiters opened but never closed, in the order opened.
///
/// Scans line by line rather than over the whole command, because the two roles
/// a line can play are mutually exclusive: a command line can open a heredoc, a
/// body line cannot. Scanning the flat string made every `<<` in a body a
/// phantom opener that never closed, so `std::cout << x`, `MASK = 1 << 8` and
/// `$((1 << 3))` inside an otherwise well-formed heredoc were all hard-rejected
/// as truncated.
///
/// Pure and side-effect free — safe to call from any tool path, including the
/// container dispatcher, which has no diagnostics engine.
pub fn unterminated_heredocs(command: &str) -> Vec<String> {
    let mut in_single = false;
    let mut in_double = false;
    let mut lines = command.lines();

    while let Some(line) = lines.next() {
        let mut open = openers_in_line(line, &mut in_single, &mut in_double);
        if open.is_empty() {
            continue;
        }

        // Bodies begin on the next line and run in the order the delimiters
        // were opened. Everything until the last terminator is file content.
        while let Some((delimiter, dash)) = open.first() {
            let Some(body) = lines.next() else {
                // Input ran out mid-body: what remains is what was cut off.
                return open.into_iter().map(|(d, _)| d).collect();
            };
            if closes(body, delimiter, *dash) {
                open.remove(0);
            }
        }

        // All closed. The next line is command text again, and a heredoc body
        // cannot leave a quote open, so the scanner resumes unquoted.
        in_single = false;
        in_double = false;
    }

    Vec::new()
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
