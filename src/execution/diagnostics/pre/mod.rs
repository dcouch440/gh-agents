//! Pre-execution checks — analyze the command string before running it.
//!
//! Each check implements [`PreCheck`] and returns an optional diagnostic
//! when a known anti-pattern is detected.

pub mod heredoc;
pub mod interactive;
pub mod shell_compat;
pub mod state_persistence;

mod tests;

use super::envelope::Diagnostic;

/// Character offsets where `pattern` occurs outside single or double quotes.
///
/// The quote state machine used to live privately inside `shell_compat`; it now
/// serves both the bash-compat checks and heredoc detection, where a `<<`
/// inside `python -c "print(1 << 2)"` must not be mistaken for a here-document.
///
/// Offsets are indices into the command's `char` sequence, not bytes.
pub(super) fn find_unquoted(cmd: &str, pattern: &str) -> Vec<usize> {
    let mut in_single = false;
    let mut in_double = false;
    find_unquoted_from(cmd, pattern, &mut in_single, &mut in_double)
}

/// [`find_unquoted`] over one line of a multi-line command, resuming the quote
/// state from the previous line and leaving it updated for the next.
///
/// Heredoc detection has to scan line by line — a `<<` inside a heredoc body is
/// file content, not another opener — but a quoted string may still span lines
/// (`python -c "` … `"`), and a `<<` inside one is a shift, not a heredoc. The
/// caller owns the state so both facts can hold at once.
pub(super) fn find_unquoted_from(
    cmd: &str,
    pattern: &str,
    in_single: &mut bool,
    in_double: &mut bool,
) -> Vec<usize> {
    let pat_chars: Vec<char> = pattern.chars().collect();
    let cmd_chars: Vec<char> = cmd.chars().collect();
    let pat_len = pat_chars.len();
    let cmd_len = cmd_chars.len();

    let mut hits = Vec::new();
    if pat_len == 0 {
        return hits;
    }
    // A line shorter than the pattern still has to walk: it may carry a lone
    // quote that flips the state the next line resumes from.

    let mut i = 0;

    while i < cmd_len {
        let c = cmd_chars[i];
        match c {
            '\\' if !*in_single && i + 1 < cmd_len => {
                i += 2;
                continue;
            }
            '\'' if !*in_double => *in_single = !*in_single,
            '"' if !*in_single => *in_double = !*in_double,
            _ if !*in_single
                && !*in_double
                && i + pat_len <= cmd_len
                && cmd_chars[i..i + pat_len] == pat_chars[..] =>
            {
                hits.push(i);
                i += pat_len;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    hits
}

/// Whether `pattern` appears outside single/double quotes.
pub(super) fn contains_unquoted(cmd: &str, pattern: &str) -> bool {
    !find_unquoted(cmd, pattern).is_empty()
}

/// A pre-execution check that analyzes the command string.
pub trait PreCheck: Send + Sync {
    /// Analyze the command and return a diagnostic if a problem is detected.
    fn check(&self, command: &str) -> Option<Diagnostic>;
}

/// Run all pre-checks against a command, collecting diagnostics.
pub fn run_pre_checks(checks: &[Box<dyn PreCheck>], command: &str) -> Vec<Diagnostic> {
    checks.iter().filter_map(|c| c.check(command)).collect()
}
