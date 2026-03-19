#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::execution::diagnostics::envelope::DiagnosticCategory;
    use crate::execution::diagnostics::post::noop::NoOpCheck;
    use crate::execution::diagnostics::post::PostCheck;
    use crate::execution::diagnostics::types::{ChangeType, FileChange};
    use crate::execution::ContainerExecResult;

    fn success_result(stdout: &str) -> ContainerExecResult {
        ContainerExecResult {
            success: true,
            exit_code: 0,
            stdout: stdout.to_string(),
            stderr: String::new(),
            duration_ms: 10,
            truncated: false,
        }
    }

    fn failed_result() -> ContainerExecResult {
        ContainerExecResult {
            success: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            duration_ms: 10,
            truncated: false,
        }
    }

    fn file_created(path: &str) -> FileChange {
        FileChange {
            path: PathBuf::from(path),
            change_type: ChangeType::Created,
            size: 100,
        }
    }

    // ── No-Op Detection ──────────────────────────────────────────────

    #[test]
    fn sed_noop_no_changes() {
        let check = NoOpCheck;
        let result = success_result("");
        let diags = check.check("sed -i 's/old/new/g' main.py", &result, &[]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].category, DiagnosticCategory::NoOp);
    }

    #[test]
    fn sed_with_changes_not_noop() {
        let check = NoOpCheck;
        let result = success_result("");
        let changes = vec![file_created("main.py")];
        let diags = check.check("sed -i 's/old/new/g' main.py", &result, &changes);
        assert!(diags.is_empty());
    }

    #[test]
    fn grep_empty_stdout_noop() {
        let check = NoOpCheck;
        let result = success_result("");
        let diags = check.check("grep -r 'pattern' src/", &result, &[]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].category, DiagnosticCategory::NoOp);
    }

    #[test]
    fn grep_with_results_not_noop() {
        let check = NoOpCheck;
        let result = success_result("src/main.py:import flask\n");
        let diags = check.check("grep -r 'flask' src/", &result, &[]);
        assert!(diags.is_empty());
    }

    #[test]
    fn failed_command_not_noop() {
        let check = NoOpCheck;
        let result = failed_result();
        let diags = check.check("sed -i 's/old/new/g' main.py", &result, &[]);
        assert!(diags.is_empty());
    }

    #[test]
    fn cp_no_changes_noop() {
        let check = NoOpCheck;
        let result = success_result("");
        let diags = check.check("cp file.txt backup/", &result, &[]);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn pip_already_installed() {
        let check = NoOpCheck;
        let result = success_result(
            "Requirement already satisfied: requests in /usr/lib/python3/dist-packages (2.28.0)\n",
        );
        let diags = check.check("pip install requests", &result, &[]);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("already installed"));
    }

    #[test]
    fn npm_up_to_date() {
        let check = NoOpCheck;
        let result = success_result("up to date, audited 50 packages in 1s\n");
        let diags = check.check("npm install express", &result, &[]);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn python_run_not_noop() {
        let check = NoOpCheck;
        let result = success_result("");
        let diags = check.check("python main.py", &result, &[]);
        assert!(diags.is_empty());
    }

    #[test]
    fn chained_cd_then_sed_noop() {
        let check = NoOpCheck;
        let result = success_result("");
        let diags = check.check("cd /app && sed -i 's/old/new/' file.py", &result, &[]);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn cat_heredoc_with_changes_not_noop() {
        let check = NoOpCheck;
        let result = success_result("");
        let changes = vec![file_created("report.md")];
        let diags = check.check("cat > report.md << 'EOF'\n# Report\nEOF", &result, &changes);
        assert!(diags.is_empty());
    }

    #[test]
    fn cat_redirect_classified_as_mutation() {
        let check = NoOpCheck;
        let result = success_result("");
        // cat > file with no changes IS a no-op (mutation with no effect)
        let diags = check.check("cat > output.txt << 'EOF'\nhello\nEOF", &result, &[]);
        assert_eq!(diags.len(), 1);
        // But it should say "no files were changed", not "search produced no results"
        assert!(diags[0].message.contains("no files were changed"));
    }

    #[test]
    fn cat_read_empty_is_search_noop() {
        let check = NoOpCheck;
        let result = success_result("");
        let diags = check.check("cat nonexistent.txt", &result, &[]);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Search produced no results"));
    }

    #[test]
    fn grep_with_file_changes_not_noop() {
        // Edge case: grep produces no stdout but files were created (side effect)
        let check = NoOpCheck;
        let result = success_result("");
        let changes = vec![file_created("some_file.log")];
        let diags = check.check("grep -r 'pattern' .", &result, &changes);
        assert!(diags.is_empty());
    }

    // ── Truncation ───────────────────────────────────────────────────

    use crate::execution::diagnostics::post::truncation::truncate_stdout;

    #[test]
    fn short_output_not_truncated() {
        let output = "line 1\nline 2\nline 3\n";
        let result = truncate_stdout("echo hello", output);
        assert_eq!(result.original_lines, 3);
        assert_eq!(result.shown_lines, 3);
        assert!(result.content.contains("line 1"));
    }

    #[test]
    fn long_output_default_head_truncated() {
        let lines: Vec<String> = (0..200).map(|i| format!("line {}", i)).collect();
        let output = lines.join("\n");
        let result = truncate_stdout("echo hello", &output);
        assert_eq!(result.original_lines, 200);
        assert_eq!(result.shown_lines, 100);
        assert!(result.content.contains("line 0"));
        assert!(!result.content.contains("line 199"));
    }

    #[test]
    fn pip_output_tail_truncated() {
        let lines: Vec<String> = (0..200).map(|i| format!("Downloading pkg-{}", i)).collect();
        let mut output = lines.join("\n");
        output.push_str("\nSuccessfully installed flask-2.3.2\n");
        let result = truncate_stdout("pip install flask", &output);
        assert!(result.shown_lines <= 30);
        assert!(result.content.contains("Successfully installed"));
    }

    #[test]
    fn cargo_test_output_parsed() {
        let mut lines = Vec::new();
        for i in 0..500 {
            lines.push(format!("test test_{} ... ok", i));
        }
        lines.push("test test_broken ... FAILED".to_string());
        lines.push("test result: FAILED. 500 passed; 1 failed; 0 ignored".to_string());
        let output = lines.join("\n");
        let result = truncate_stdout("cargo test", &output);
        assert!(result.content.contains("FAILED"));
        assert!(result.content.contains("test result:"));
        assert!(result.shown_lines < 200);
    }

    #[test]
    fn ansi_codes_stripped() {
        let output = "\x1b[32mPASS\x1b[0m test_foo\n\x1b[31mFAIL\x1b[0m test_bar\n";
        let result = truncate_stdout("jest", output);
        assert!(result.content.contains("PASS"));
        assert!(!result.content.contains("\x1b"));
    }

    // ── Stderr Classification ────────────────────────────────────────

    use crate::execution::diagnostics::post::stderr_classifier::classify_stderr;

    #[test]
    fn error_line_classified() {
        let classified = classify_stderr("error: something went wrong\n");
        assert_eq!(classified.errors.len(), 1);
        assert_eq!(classified.warnings.len(), 0);
    }

    #[test]
    fn warning_line_classified() {
        let classified = classify_stderr("npm WARN deprecated uuid@3.4.0\n");
        assert_eq!(classified.errors.len(), 0);
        assert_eq!(classified.warnings.len(), 1);
    }

    #[test]
    fn mixed_classification() {
        let stderr = "warning: unused variable\nerror[E0308]: mismatched types\nnote: some note\n";
        let classified = classify_stderr(stderr);
        assert_eq!(classified.errors.len(), 1);
        assert_eq!(classified.warnings.len(), 2); // warning + note
        assert!(classified.summary.contains("1 error"));
        assert!(classified.summary.contains("2 warning"));
    }

    #[test]
    fn python_traceback_is_error() {
        let classified = classify_stderr("Traceback (most recent call last):\n");
        assert_eq!(classified.errors.len(), 1);
    }

    #[test]
    fn module_not_found_is_error() {
        let classified = classify_stderr("ModuleNotFoundError: No module named 'requests'\n");
        assert_eq!(classified.errors.len(), 1);
    }

    #[test]
    fn command_not_found_is_error() {
        let classified = classify_stderr("sh: command not found: jq\n");
        assert_eq!(classified.errors.len(), 1);
    }

    // ── Suggestions ──────────────────────────────────────────────────

    use crate::execution::diagnostics::post::suggestions::suggest_fix;

    #[test]
    fn suggest_install_for_command_not_found() {
        let diags = suggest_fix("command not found: jq");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("apt-get install -y jq"));
    }

    #[test]
    fn suggest_pip_install_for_module_not_found() {
        let diags = suggest_fix("ModuleNotFoundError: No module named 'requests'");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("pip install requests"));
    }

    #[test]
    fn suggest_npm_install_for_cannot_find_module() {
        let diags = suggest_fix("Cannot find module 'express'");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("npm install express"));
    }

    #[test]
    fn suggest_chmod_for_permission_denied() {
        let diags = suggest_fix("Permission denied: ./run.sh");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("chmod +x"));
    }

    #[test]
    fn no_suggestion_for_clean_stderr() {
        let diags = suggest_fix("All tests passed.\n");
        assert!(diags.is_empty());
    }
}
