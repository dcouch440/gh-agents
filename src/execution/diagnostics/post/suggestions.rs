//! Fix suggestions — thefuck-style pattern matching for common errors.
//!
//! Each rule matches a stderr pattern and suggests a fix command.

use std::sync::LazyLock;

use regex::Regex;

use super::super::envelope::{Diagnostic, DiagnosticCategory, Severity};

/// Check stderr for known error patterns and suggest fixes.
pub fn suggest_fix(stderr: &str) -> Vec<Diagnostic> {
    RULES
        .iter()
        .filter_map(|rule| {
            rule.pattern.captures(stderr).map(|caps| Diagnostic {
                severity: Severity::Info,
                category: DiagnosticCategory::Suggestion,
                message: (rule.suggest)(&caps),
                suggestion: None,
            })
        })
        .collect()
}

struct SuggestionRule {
    pattern: Regex,
    suggest: fn(&regex::Captures) -> String,
}

static RULES: LazyLock<Vec<SuggestionRule>> = LazyLock::new(|| {
    vec![
        SuggestionRule {
            pattern: Regex::new(r"(?i)command not found:\s*(\S+)").unwrap(),
            suggest: |caps| format!("Install with: apt-get install -y {}", &caps[1]),
        },
        SuggestionRule {
            pattern: Regex::new(r#"(?i)No such file or directory:\s*['"]*([^'":\n]+)"#).unwrap(),
            suggest: |caps| {
                let path = caps[1].trim();
                if let Some(parent) = std::path::Path::new(path).parent() {
                    if parent.as_os_str().is_empty() {
                        "File doesn't exist. Check the path.".to_string()
                    } else {
                        format!("File doesn't exist. Check: ls {}", parent.display())
                    }
                } else {
                    "File doesn't exist. Check the path.".to_string()
                }
            },
        },
        SuggestionRule {
            pattern: Regex::new(r#"(?i)Permission denied:\s*['"]*([^'":\n]+)"#).unwrap(),
            suggest: |caps| format!("Try: chmod +x {}", caps[1].trim()),
        },
        SuggestionRule {
            pattern: Regex::new(r"(?i)ModuleNotFoundError: No module named '(\w+)'").unwrap(),
            suggest: |caps| format!("Install with: pip install {}", &caps[1]),
        },
        SuggestionRule {
            pattern: Regex::new(r"(?i)Cannot find module '([^']+)'").unwrap(),
            suggest: |caps| format!("Install with: npm install {}", &caps[1]),
        },
        SuggestionRule {
            pattern: Regex::new(r"(?i)Address already in use").unwrap(),
            suggest: |_| {
                "Port already in use. Kill the existing process or use a different port."
                    .to_string()
            },
        },
        SuggestionRule {
            pattern: Regex::new(r"(?i)database is locked").unwrap(),
            suggest: |_| "SQLite database is locked. Another process may have it open.".to_string(),
        },
    ]
});
