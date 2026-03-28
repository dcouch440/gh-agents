//! Smart output truncation — context-aware truncation that keeps signal.
//!
//! Instead of blindly cutting at N bytes, this module applies strategies
//! based on command type: tail-first for build tools (errors at bottom),
//! test parsers for test runners, and head-first as default.

/// Maximum lines of output to show (SWE-Agent empirical: ~100).
const MAX_OUTPUT_LINES: usize = 100;

/// Result of truncation.
pub struct TruncatedOutput {
    /// The (possibly truncated) content.
    pub content: String,
    /// Number of lines in the original output.
    pub original_lines: usize,
    /// Number of lines shown.
    pub shown_lines: usize,
}

impl TruncatedOutput {
    /// Summary string for the stdout header.
    pub fn summary(&self) -> String {
        if self.original_lines <= self.shown_lines {
            format!("{} lines", self.original_lines)
        } else {
            format!(
                "showing last {} of {} lines",
                self.shown_lines, self.original_lines
            )
        }
    }
}

/// Truncate stdout based on the command type.
pub fn truncate_stdout(command: &str, stdout: &str) -> TruncatedOutput {
    let stripped = strip_ansi(stdout);
    let lines: Vec<&str> = stripped.lines().collect();
    let original_lines = lines.len();

    if original_lines <= MAX_OUTPUT_LINES {
        return TruncatedOutput {
            content: stripped,
            original_lines,
            shown_lines: original_lines,
        };
    }

    let strategy = detect_strategy(command);
    match strategy {
        Strategy::TailFirst(n) => {
            let keep = n.min(original_lines);
            let kept: Vec<&str> = lines[original_lines - keep..].to_vec();
            TruncatedOutput {
                content: format!("...\n{}", kept.join("\n")),
                original_lines,
                shown_lines: keep,
            }
        }
        Strategy::TestParser(runner) => parse_test_output(&stripped, runner, original_lines),
        Strategy::Default => {
            let keep = MAX_OUTPUT_LINES.min(original_lines);
            let kept: Vec<&str> = lines[..keep].to_vec();
            TruncatedOutput {
                content: format!("{}\n...", kept.join("\n")),
                original_lines,
                shown_lines: keep,
            }
        }
    }
}

enum Strategy {
    TailFirst(usize),
    TestParser(TestRunner),
    Default,
}

#[derive(Clone, Copy)]
enum TestRunner {
    CargoTest,
    Pytest,
    Jest,
}

fn detect_strategy(command: &str) -> Strategy {
    let lower = command.to_lowercase();
    let first = command
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();

    match first.as_str() {
        "npm" | "yarn" | "pip" | "pip3" => Strategy::TailFirst(30),
        "make" => Strategy::TailFirst(30),
        "cargo" => {
            if lower.contains("test") {
                Strategy::TestParser(TestRunner::CargoTest)
            } else {
                Strategy::TailFirst(50)
            }
        }
        "pytest" | "python" if lower.contains("pytest") => Strategy::TestParser(TestRunner::Pytest),
        "jest" | "vitest" | "npx" if lower.contains("test") || lower.contains("vitest") => {
            Strategy::TestParser(TestRunner::Jest)
        }
        _ => Strategy::Default,
    }
}

/// Parse test runner output to extract summary + failures only.
fn parse_test_output(output: &str, runner: TestRunner, original_lines: usize) -> TruncatedOutput {
    let (summary, failures) = match runner {
        TestRunner::CargoTest => parse_cargo_test(output),
        TestRunner::Pytest => parse_pytest(output),
        TestRunner::Jest => parse_jest(output),
    };

    let mut content = String::new();
    if let Some(s) = summary {
        content.push_str(&s);
    }
    if !failures.is_empty() {
        if !content.is_empty() {
            content.push_str("\n\n");
        }
        content.push_str("Failed:\n");
        for f in &failures {
            content.push_str("  ");
            content.push_str(f);
            content.push('\n');
        }
    }

    if content.is_empty() {
        // Fallback: couldn't parse, use tail
        let lines: Vec<&str> = output.lines().collect();
        let keep = 50.min(lines.len());
        return TruncatedOutput {
            content: format!("...\n{}", lines[lines.len() - keep..].join("\n")),
            original_lines,
            shown_lines: keep,
        };
    }

    let shown = content.lines().count();
    TruncatedOutput {
        content,
        original_lines,
        shown_lines: shown,
    }
}

fn parse_cargo_test(output: &str) -> (Option<String>, Vec<String>) {
    let mut summary = None;
    let mut failures = Vec::new();

    for line in output.lines() {
        if line.starts_with("test result:") {
            summary = Some(line.to_string());
        }
        if line.contains("FAILED") && line.starts_with("test ") {
            failures.push(line.trim().to_string());
        }
    }
    (summary, failures)
}

fn parse_pytest(output: &str) -> (Option<String>, Vec<String>) {
    let mut summary = None;
    let mut failures = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        // Summary line: "= 5 passed, 2 failed in 1.23s ="
        if trimmed.contains("passed")
            && (trimmed.contains("failed") || trimmed.contains("error"))
            && (trimmed.starts_with('=') || trimmed.ends_with('='))
        {
            summary = Some(trimmed.to_string());
        }
        // Failed test line: "FAILED tests/test_foo.py::test_bar"
        if trimmed.starts_with("FAILED ") {
            failures.push(trimmed.to_string());
        }
    }
    (summary, failures)
}

fn parse_jest(output: &str) -> (Option<String>, Vec<String>) {
    let mut summary = None;
    let mut failures = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        // "Tests:       2 failed, 5 passed, 7 total"
        if trimmed.starts_with("Tests:") {
            summary = Some(trimmed.to_string());
        }
        // "FAIL src/foo.test.ts"
        if trimmed.starts_with("FAIL ") {
            failures.push(trimmed.to_string());
        }
    }
    (summary, failures)
}

/// Strip ANSI escape sequences from output.
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ESC[...m sequences
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
        }
        result.push(c);
    }
    result
}
