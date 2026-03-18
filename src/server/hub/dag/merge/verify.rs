//! Programmatic verification of LLM merge resolutions.
//!
//! Checks that merged output is non-empty, reasonable length,
//! preserves imports, and maintains syntactic validity.

use super::types::{FileType, Language, StructuredKind};

/// Outcome of verifying a merge resolution.
#[derive(Debug)]
pub enum VerifyOutcome {
    Ok,
    Warning(String),
    Failed(String),
}

/// Verify that a resolved merge hunk is valid.
pub fn verify_resolution(
    merged: &str,
    version_a: &str,
    version_b: &str,
    file_type: &FileType,
) -> VerifyOutcome {
    // 1. Non-empty check
    if merged.trim().is_empty() {
        return VerifyOutcome::Failed("Merged output is empty".to_string());
    }

    // 2. Length sanity — merged should be between min/3 and max*3
    let min_len = version_a.len().min(version_b.len());
    let max_len = version_a.len().max(version_b.len());

    if max_len > 0 && merged.len() > max_len * 3 {
        return VerifyOutcome::Failed(format!(
            "Merged output suspiciously long ({} chars vs max input {})",
            merged.len(),
            max_len
        ));
    }
    if min_len > 50 && merged.len() < min_len / 3 {
        return VerifyOutcome::Failed(format!(
            "Merged output suspiciously short ({} chars vs min input {})",
            merged.len(),
            min_len
        ));
    }

    // 3. Import completeness (code files)
    if let FileType::Code(ref lang) = file_type {
        let imports_a = count_imports(version_a, lang);
        let imports_b = count_imports(version_b, lang);
        let imports_merged = count_imports(merged, lang);
        let expected = imports_a.max(imports_b);
        if expected > 0 && imports_merged < expected {
            return VerifyOutcome::Warning(format!(
                "Fewer imports than expected: {} in merged vs {} expected (A={}, B={})",
                imports_merged, expected, imports_a, imports_b
            ));
        }
    }

    // 4. Bracket balance (code files — heuristic)
    if matches!(file_type, FileType::Code(_)) && !brackets_balanced(merged) {
        return VerifyOutcome::Warning("Bracket imbalance in merged output".to_string());
    }

    // 5. JSON validity (structured data)
    if matches!(file_type, FileType::Structured(StructuredKind::Json)) {
        // Only check if the merged content looks like a complete JSON value
        let trimmed = merged.trim();
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            if serde_json::from_str::<serde_json::Value>(trimmed).is_err() {
                return VerifyOutcome::Failed("Invalid JSON in merged output".to_string());
            }
        }
    }

    VerifyOutcome::Ok
}

/// Count import-like lines in a code fragment.
fn count_imports(content: &str, lang: &Language) -> usize {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return false;
            }
            match lang {
                Language::Python => trimmed.starts_with("import ") || trimmed.starts_with("from "),
                Language::JavaScript | Language::TypeScript => {
                    trimmed.starts_with("import ")
                        || (trimmed.starts_with("const ") && trimmed.contains("require("))
                }
                Language::Rust => trimmed.starts_with("use ") || trimmed.starts_with("pub use "),
                Language::Go => trimmed.starts_with("import ") || trimmed.starts_with('"'),
                _ => trimmed.starts_with("import ") || trimmed.starts_with("from "),
            }
        })
        .count()
}

/// Heuristic bracket balance check.
fn brackets_balanced(content: &str) -> bool {
    let mut parens = 0i32;
    let mut brackets = 0i32;
    let mut braces = 0i32;
    let mut in_string = false;
    let mut string_char = '"';
    let mut prev = '\0';

    for ch in content.chars() {
        if in_string {
            if ch == string_char && prev != '\\' {
                in_string = false;
            }
            prev = ch;
            continue;
        }

        match ch {
            '"' | '\'' | '`' => {
                in_string = true;
                string_char = ch;
            }
            '(' => parens += 1,
            ')' => parens -= 1,
            '[' => brackets += 1,
            ']' => brackets -= 1,
            '{' => braces += 1,
            '}' => braces -= 1,
            _ => {}
        }

        prev = ch;

        // Early bail on negative (more closing than opening)
        if parens < 0 || brackets < 0 || braces < 0 {
            return false;
        }
    }

    parens == 0 && brackets == 0 && braces == 0
}
