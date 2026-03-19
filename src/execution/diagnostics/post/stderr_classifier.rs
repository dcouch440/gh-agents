//! Stderr classification — separates errors from warnings from noise.
//!
//! Agents see `WARN` in stderr and panic, chasing harmless deprecation
//! warnings. Classification lets them focus on actual errors.

/// Classified stderr output.
pub struct ClassifiedStderr {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    /// Summary line: "2 warnings (non-blocking), 1 error"
    pub summary: String,
}

/// Classify each stderr line as error, warning, or noise.
pub fn classify_stderr(stderr: &str) -> ClassifiedStderr {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if is_error_line(trimmed) {
            errors.push(trimmed.to_string());
        } else if is_warning_line(trimmed) {
            warnings.push(trimmed.to_string());
        }
        // Noise lines are dropped entirely
    }

    let summary = format!(
        "{} error{}, {} warning{}",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" },
        warnings.len(),
        if warnings.len() == 1 { "" } else { "s" },
    );

    ClassifiedStderr {
        errors,
        warnings,
        summary,
    }
}

fn is_error_line(line: &str) -> bool {
    let lower = line.to_lowercase();

    // Explicit error markers
    if lower.starts_with("error:")
        || lower.starts_with("error[")
        || lower.contains("] error:")
        || lower.starts_with("fatal:")
    {
        return true;
    }

    // Python tracebacks and exceptions
    if lower.starts_with("traceback")
        || lower.contains("syntaxerror:")
        || lower.contains("typeerror:")
        || lower.contains("valueerror:")
        || lower.contains("keyerror:")
        || lower.contains("importerror:")
        || lower.contains("modulenotfounderror:")
        || lower.contains("filenotfounderror:")
        || lower.contains("nameerror:")
        || lower.contains("attributeerror:")
        || lower.contains("indexerror:")
        || lower.contains("oserror:")
    {
        return true;
    }

    // Node.js / general
    if lower.contains("cannot find module")
        || lower.contains("module not found")
        || lower.contains("no such file or directory")
        || lower.contains("permission denied")
        || lower.contains("command not found")
        || lower.contains("connection refused")
        || lower.contains("connection timed out")
    {
        return true;
    }

    // Node.js error codes
    if lower.contains("enoent")
        || lower.contains("eacces")
        || lower.contains("econnrefused")
        || lower.contains("eaddrinuse")
    {
        return true;
    }

    // Rust panics
    if lower.starts_with("panic:") || lower.contains("thread '") && lower.contains("panicked") {
        return true;
    }

    false
}

fn is_warning_line(line: &str) -> bool {
    let lower = line.to_lowercase();

    lower.starts_with("warning:")
        || lower.starts_with("warn:")
        || lower.contains("] warn ")
        || lower.contains("] warning:")
        || lower.contains("deprecationwarning:")
        || lower.contains("futurewarning:")
        || lower.contains("pendingdeprecationwarning:")
        || lower.contains("userwarning:")
        || lower.starts_with("npm warn")
        || lower.starts_with("npm notice")
        || lower.contains("pip warning:")
        || lower.starts_with("note:")
}
