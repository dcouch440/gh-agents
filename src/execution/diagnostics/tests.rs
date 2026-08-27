#[cfg(test)]
mod tests {
    use crate::execution::diagnostics::envelope::{
        CommandEnvelope, Diagnostic, DiagnosticCategory, Severity,
    };
    use crate::execution::diagnostics::html_unescape;
    use crate::execution::diagnostics::loop_detector::LoopStatus;

    #[test]
    fn render_failed_command() {
        let envelope = CommandEnvelope {
            command: "jq . file.json".to_string(),
            exit_code: 1,
            stdout: String::new(),
            stderr: "command not found: jq\n".to_string(),
            duration_ms: 5,
            severity: Severity::Error,
            pre_warnings: vec![],
            post_diagnostics: vec![],
            file_changes: vec![],
            workspace_digest: None,
            loop_status: LoopStatus::Clean,
        };
        let rendered = envelope.render();
        assert!(rendered.starts_with("result: failed\n"));
        assert!(rendered.contains("stderr summary:"));
        assert!(rendered.contains("[ERROR]"));
    }

    #[test]
    fn render_with_post_diagnostics() {
        let envelope = CommandEnvelope {
            command: "sed -i 's/old/new/' file.py".to_string(),
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 10,
            severity: Severity::NoOp,
            pre_warnings: vec![],
            post_diagnostics: vec![Diagnostic {
                severity: Severity::NoOp,
                category: DiagnosticCategory::NoOp,
                message: "sed made 0 replacements.".to_string(),
                suggestion: Some("Check the pattern.".to_string()),
            }],
            file_changes: vec![],
            workspace_digest: None,
            loop_status: LoopStatus::Clean,
        };
        let rendered = envelope.render();
        assert!(rendered.contains("result: success (no-op)"));
        assert!(rendered.contains("sed made 0 replacements"));
        assert!(rendered.contains("suggestion: Check the pattern"));
    }

    #[test]
    fn render_with_file_changes() {
        use crate::execution::diagnostics::types::{ChangeType, FileChange};
        use std::path::PathBuf;

        let envelope = CommandEnvelope {
            command: "python build.py".to_string(),
            exit_code: 0,
            stdout: "done\n".to_string(),
            stderr: String::new(),
            duration_ms: 100,
            severity: Severity::Ok,
            pre_warnings: vec![],
            post_diagnostics: vec![],
            file_changes: vec![
                FileChange {
                    path: PathBuf::from("src/main.py"),
                    change_type: ChangeType::Created,
                    size: 420,
                },
                FileChange {
                    path: PathBuf::from("config.json"),
                    change_type: ChangeType::Modified,
                    size: 150,
                },
            ],
            workspace_digest: None,
            loop_status: LoopStatus::Clean,
        };
        let rendered = envelope.render();
        assert!(rendered.contains("changes:"));
        assert!(rendered.contains("created: src/main.py"));
        assert!(rendered.contains("modified: config.json"));
    }

    #[test]
    fn html_unescape_entities() {
        assert_eq!(
            html_unescape("cat &gt; file.py &lt;&lt; &#39;EOF&#39;"),
            "cat > file.py << 'EOF'"
        );
        assert_eq!(
            html_unescape("echo &quot;hello&quot; &amp;&amp; exit"),
            "echo \"hello\" && exit"
        );
        assert_eq!(html_unescape("echo &#x27;world&#x27;"), "echo 'world'");
        assert_eq!(html_unescape("brother&apos;s form"), "brother's form");
    }

    #[test]
    fn html_unescape_no_entities() {
        assert_eq!(html_unescape("ls -la"), "ls -la");
    }

    #[test]
    fn strip_grok_citation_tags() {
        let input = "Hello<grok:render type=\"render_inline_citation\">\n<argument name=\"citation_id\">42</argument>\n</grok:render> world";
        assert_eq!(html_unescape(input), "Hello world");
    }

    #[test]
    fn strip_multiple_grok_tags() {
        let input = "A<grok:render type=\"x\">\n<argument name=\"citation_id\">1</argument>\n</grok:render> B<grok:render type=\"y\">\n<argument name=\"citation_id\">2</argument>\n</grok:render> C";
        assert_eq!(html_unescape(input), "A B C");
    }

    #[test]
    fn strip_grok_tags_in_heredoc() {
        let input = "cat > f.md << 'EOF'\n# Title<grok:render type=\"render_inline_citation\">\n<argument name=\"citation_id\">5</argument>\n</grok:render>\nContent here.\nEOF";
        let result = html_unescape(input);
        assert!(!result.contains("grok:render"));
        assert!(result.contains("# Title"));
        assert!(result.contains("Content here."));
        assert!(result.contains("EOF"));
    }

    #[test]
    fn no_grok_tags_unchanged() {
        let input = "cat > file.py << 'EOF'\nprint('hello')\nEOF";
        assert_eq!(html_unescape(input), input);
    }

    #[test]
    fn render_full_envelope() {
        use crate::execution::diagnostics::types::{ChangeType, FileChange};
        use crate::execution::diagnostics::workspace::digest::WorkspaceDigest;
        use std::path::PathBuf;

        let envelope = CommandEnvelope {
            command: "python build.py".to_string(),
            exit_code: 0,
            stdout: "Building...\nDone.\n".to_string(),
            stderr: "warning: unused import\n".to_string(),
            duration_ms: 200,
            severity: Severity::Ok,
            pre_warnings: vec![],
            post_diagnostics: vec![],
            file_changes: vec![FileChange {
                path: PathBuf::from("dist/app.js"),
                change_type: ChangeType::Created,
                size: 5000,
            }],
            workspace_digest: Some(WorkspaceDigest {
                file_count: 10,
                file_delta: 1,
                dir_count: 3,
                total_size: 50_000,
                last_modified: Some(PathBuf::from("dist/app.js")),
            }),
            loop_status: LoopStatus::Clean,
        };
        let rendered = envelope.render();
        // Should contain all sections
        assert!(rendered.contains("result: success"));
        assert!(rendered.contains("stdout"));
        assert!(rendered.contains("stderr summary:"));
        assert!(rendered.contains("changes:"));
        assert!(rendered.contains("created: dist/app.js"));
        // Digest renders because file_delta != 0.
        assert!(rendered.contains("10 files (+1)"));
        assert!(!rendered.contains("last: dist/app.js"));
    }
    #[test]
    fn digest_is_suppressed_when_workspace_did_not_move() {
        use crate::execution::diagnostics::workspace::digest::WorkspaceDigest;

        let envelope = CommandEnvelope {
            command: "cat notes.md".into(),
            exit_code: 0,
            stdout: "some notes".into(),
            stderr: String::new(),
            duration_ms: 3,
            severity: Severity::Ok,
            pre_warnings: vec![],
            post_diagnostics: vec![],
            file_changes: vec![],
            workspace_digest: Some(WorkspaceDigest {
                file_count: 12,
                file_delta: 0,
                dir_count: 3,
                total_size: 48_000,
                last_modified: None,
            }),
            loop_status: LoopStatus::Clean,
        };

        let rendered = envelope.render();
        assert!(rendered.contains("some notes"));
        // A read-only command should not re-print workspace scaffolding.
        assert!(!rendered.contains("12 files"));
    }

    #[test]
    fn digest_still_renders_when_command_failed() {
        use crate::execution::diagnostics::workspace::digest::WorkspaceDigest;

        let envelope = CommandEnvelope {
            command: "python build.py".into(),
            exit_code: 1,
            stdout: String::new(),
            stderr: "Traceback".into(),
            duration_ms: 40,
            severity: Severity::Error,
            pre_warnings: vec![],
            post_diagnostics: vec![],
            file_changes: vec![],
            workspace_digest: Some(WorkspaceDigest {
                file_count: 12,
                file_delta: 0,
                dir_count: 3,
                total_size: 48_000,
                last_modified: None,
            }),
            loop_status: LoopStatus::Clean,
        };

        // A failing command is exactly when re-orientation helps.
        assert!(envelope.render().contains("12 files"));
    }

    #[test]
    fn produced_files_drops_noise_and_deletions() {
        use crate::execution::diagnostics::workspace::is_noise;
        use std::path::Path;

        assert!(is_noise(Path::new(".git/config")));
        assert!(is_noise(Path::new(
            "venv/lib/site-packages/requests/api.py"
        )));
        assert!(is_noise(Path::new("src/__pycache__/mod.cpython-311.pyc")));
        assert!(is_noise(Path::new("node_modules/left-pad/index.js")));

        // Real deliverables survive.
        assert!(!is_noise(Path::new("pricing_2026.md")));
        assert!(!is_noise(Path::new("reports/analysis.md")));
    }
}
