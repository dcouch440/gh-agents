#[cfg(test)]
mod tests {
    use super::super::*;

    // ── ContainerConfig defaults ──────────────────────────────────────────

    #[test]
    fn container_config_defaults() {
        let config = ContainerConfig::default();
        assert_eq!(config.image, crate::constants::CONTAINER_DEFAULT_IMAGE);
        assert_eq!(
            config.memory_limit,
            crate::constants::CONTAINER_DEFAULT_MEMORY
        );
        assert_eq!(config.cpu_limit, crate::constants::CONTAINER_DEFAULT_CPUS);
        assert_eq!(
            config.command_timeout_secs,
            crate::constants::CONTAINER_COMMAND_TIMEOUT_SECS
        );
        assert_eq!(config.workdir, "/workspace");
        assert!(config.clone_url.is_empty());
        assert!(config.github_token.is_empty());
        assert!(config.branch.is_none());
        assert!(config.env_vars.is_empty());
        assert!(config.workspace_volume.is_none());
    }

    // ── ContainerExecResult ───────────────────────────────────────────────

    #[test]
    fn container_exec_result_success_check() {
        let result = ContainerExecResult {
            success: true,
            exit_code: 0,
            stdout: "ok".to_string(),
            stderr: String::new(),
            duration_ms: 100,
            truncated: false,
        };
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert!(!result.truncated);
    }

    #[test]
    fn container_exec_result_failure_check() {
        let result = ContainerExecResult {
            success: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            duration_ms: 50,
            truncated: false,
        };
        assert!(!result.success);
        assert_eq!(result.exit_code, 1);
    }

    // ── ContainerError display ────────────────────────────────────────────

    #[test]
    fn container_error_display() {
        let err = ContainerError::CloneFailed {
            stderr: "auth failed".to_string(),
        };
        assert!(err.to_string().contains("auth failed"));

        let err = ContainerError::CommandFailed {
            container: "test-123".to_string(),
            exit_code: 127,
            stderr: "not found".to_string(),
        };
        assert!(err.to_string().contains("test-123"));
        assert!(err.to_string().contains("127"));

        let err = ContainerError::Timeout {
            container: "test-456".to_string(),
            timeout_secs: 300,
        };
        assert!(err.to_string().contains("300"));
    }

    #[test]
    fn container_error_path_not_allowed_display() {
        let err = ContainerError::PathNotAllowed {
            path: "/etc/shadow".to_string(),
            reason: "absolute paths are not allowed".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/etc/shadow"));
        assert!(msg.contains("absolute paths"));
    }

    #[test]
    fn container_error_docker_spawn_failed_display() {
        let err = ContainerError::DockerSpawnFailed {
            operation: "exec",
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "docker not found"),
        };
        let msg = err.to_string();
        assert!(msg.contains("exec"));
        assert!(msg.contains("docker not found"));
    }

    // ── RedactedString ────────────────────────────────────────────────────

    #[test]
    fn redacted_string_hides_in_debug() {
        let secret = RedactedString::new("ghp_super_secret_token_12345");
        let debug_output = format!("{:?}", secret);
        assert_eq!(debug_output, "[REDACTED]");
        assert!(!debug_output.contains("ghp_"));
    }

    #[test]
    fn redacted_string_hides_in_display() {
        let secret = RedactedString::new("ghp_super_secret_token_12345");
        let display_output = format!("{}", secret);
        assert_eq!(display_output, "[REDACTED]");
        assert!(!display_output.contains("ghp_"));
    }

    #[test]
    fn redacted_string_expose_returns_value() {
        let token = "ghp_super_secret_token_12345";
        let secret = RedactedString::new(token);
        assert_eq!(secret.expose(), token);
    }

    #[test]
    fn redacted_string_is_empty() {
        assert!(RedactedString::default().is_empty());
        assert!(RedactedString::new("").is_empty());
        assert!(!RedactedString::new("something").is_empty());
    }

    #[test]
    fn redacted_string_clone() {
        let original = RedactedString::new("secret");
        let cloned = original.clone();
        assert_eq!(cloned.expose(), "secret");
    }

    // ── sanitize_git_output ───────────────────────────────────────────────

    #[test]
    fn sanitize_git_output_strips_token() {
        let token = RedactedString::new("ghp_abc123xyz");
        let stderr = "fatal: Authentication failed for 'https://x-access-token:ghp_abc123xyz@github.com/owner/repo.git/'";
        let sanitized = sanitize_git_output(stderr, &token);
        assert!(!sanitized.contains("ghp_abc123xyz"));
        assert!(sanitized.contains("[REDACTED]"));
        assert!(sanitized.contains("Authentication failed"));
    }

    #[test]
    fn sanitize_git_output_no_token_passthrough() {
        let token = RedactedString::new("ghp_abc123xyz");
        let stderr = "fatal: repository not found";
        let sanitized = sanitize_git_output(stderr, &token);
        assert_eq!(sanitized, stderr);
    }

    #[test]
    fn sanitize_git_output_empty_token_passthrough() {
        let token = RedactedString::default();
        let stderr = "fatal: some error with ghp_something";
        let sanitized = sanitize_git_output(stderr, &token);
        assert_eq!(sanitized, stderr);
    }

    #[test]
    fn sanitize_git_output_multiple_occurrences() {
        let token = RedactedString::new("secret123");
        let stderr = "token=secret123 retry with secret123";
        let sanitized = sanitize_git_output(stderr, &token);
        assert!(!sanitized.contains("secret123"));
        assert_eq!(
            sanitized.matches("[REDACTED]").count(),
            2,
            "Both occurrences should be replaced"
        );
    }

    // ── shell_escape_path ─────────────────────────────────────────────────

    #[test]
    fn shell_escape_simple_path() {
        assert_eq!(shell_escape_path("src/main.rs"), "'src/main.rs'");
    }

    #[test]
    fn shell_escape_path_with_spaces() {
        assert_eq!(
            shell_escape_path("my project/file name.txt"),
            "'my project/file name.txt'"
        );
    }

    #[test]
    fn shell_escape_path_with_single_quote() {
        // can't.rs → 'can'\''t.rs'
        let escaped = shell_escape_path("can't.rs");
        assert_eq!(escaped, "'can'\\''t.rs'");
        assert!(!escaped.contains("can't")); // original quote pattern is broken
    }

    #[test]
    fn shell_escape_path_with_multiple_quotes() {
        let escaped = shell_escape_path("it's a 'test'");
        assert!(!escaped.contains("it's")); // quotes are properly escaped
    }

    #[test]
    fn shell_escape_path_with_special_chars() {
        // Dollar signs, backticks etc should be safe inside single quotes
        let escaped = shell_escape_path("file$var.txt");
        assert_eq!(escaped, "'file$var.txt'");
    }

    // ── validate_container_path ───────────────────────────────────────────

    #[test]
    fn validate_container_path_allows_simple_relative() {
        assert!(validate_container_path("src/main.rs").is_ok());
    }

    #[test]
    fn validate_container_path_allows_current_dir_relative() {
        assert!(validate_container_path("./src/main.rs").is_ok());
    }

    #[test]
    fn validate_container_path_allows_simple_filename() {
        assert!(validate_container_path("file.rs").is_ok());
    }

    #[test]
    fn validate_container_path_rejects_absolute() {
        let result = validate_container_path("/etc/shadow");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn validate_container_path_rejects_traversal() {
        let result = validate_container_path("../../../etc/passwd");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("traversal"));
    }

    #[test]
    fn validate_container_path_rejects_hidden_traversal() {
        let result = validate_container_path("src/../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn validate_container_path_allows_dotfiles() {
        // Single dots and dotfiles should be fine
        assert!(validate_container_path(".gitignore").is_ok());
        assert!(validate_container_path("src/.env.example").is_ok());
    }

    // ── ContainerConfig with RedactedString ───────────────────────────────

    #[test]
    fn container_config_token_not_leaked_in_debug() {
        let config = ContainerConfig {
            github_token: RedactedString::new("ghp_secret123"),
            ..ContainerConfig::default()
        };
        let debug = format!("{:?}", config);
        assert!(!debug.contains("ghp_secret123"));
        assert!(debug.contains("[REDACTED]"));
    }

    // ── truncate_output ───────────────────────────────────────────────────

    #[test]
    fn truncate_output_no_op_when_under_limit() {
        let input = b"hello world";
        let (result, truncated) = truncate_output(input, 1024);
        assert_eq!(result, "hello world");
        assert!(!truncated);
    }

    #[test]
    fn truncate_output_exact_limit() {
        let input = b"12345";
        let (result, truncated) = truncate_output(input, 5);
        assert_eq!(result, "12345");
        assert!(!truncated);
    }

    #[test]
    fn truncate_output_truncates_large_input() {
        let input = b"hello world, this is a long string";
        let (result, truncated) = truncate_output(input, 5);
        assert!(truncated);
        assert!(result.starts_with("hello"));
        assert!(result.contains("[truncated,"));
        assert!(result.contains("bytes total"));
        assert!(result.contains(&input.len().to_string()));
    }

    #[test]
    fn truncate_output_preserves_size_info() {
        let input = vec![b'x'; 10_000];
        let (result, truncated) = truncate_output(&input, 100);
        assert!(truncated);
        assert!(result.contains("10000 bytes total"));
    }

    // ── parse_docker_timestamp ────────────────────────────────────────────

    #[test]
    fn parse_docker_timestamp_standard_format() {
        let ts = "2024-01-15 10:30:00 -0700 MST";
        let parsed = parse_docker_timestamp(ts);
        assert!(parsed.is_some());
        let dt = parsed.unwrap();
        assert_eq!(dt.date_naive().to_string(), "2024-01-15");
    }

    #[test]
    fn parse_docker_timestamp_utc() {
        let ts = "2024-06-01 00:00:00 +0000 UTC";
        let parsed = parse_docker_timestamp(ts);
        assert!(parsed.is_some());
        let dt = parsed.unwrap();
        assert_eq!(dt.date_naive().to_string(), "2024-06-01");
    }

    #[test]
    fn parse_docker_timestamp_positive_offset() {
        let ts = "2024-03-20 15:45:30 +0530 IST";
        let parsed = parse_docker_timestamp(ts);
        assert!(parsed.is_some());
    }

    #[test]
    fn parse_docker_timestamp_invalid() {
        assert!(parse_docker_timestamp("not a timestamp").is_none());
        assert!(parse_docker_timestamp("").is_none());
        assert!(parse_docker_timestamp("2024-01-15").is_none());
    }

    // ── network isolation ─────────────────────────────────────────────────

    #[test]
    fn container_config_network_isolated_default_true() {
        let config = ContainerConfig::default();
        assert!(config.network_isolated);
    }

    #[test]
    fn container_config_network_isolated_can_be_disabled() {
        let config = ContainerConfig {
            network_isolated: false,
            ..ContainerConfig::default()
        };
        assert!(!config.network_isolated);
    }

    #[test]
    fn network_disconnect_error_display() {
        let err = ContainerError::NetworkDisconnectFailed {
            container: "nexor-step-abc".to_string(),
            stderr: "not connected".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("nexor-step-abc"));
        assert!(msg.contains("not connected"));
    }

    // ── ContainerError::CreateTimeout ────────────────────────────────────

    #[test]
    fn container_error_create_timeout_display() {
        let err = ContainerError::CreateTimeout {
            container: "nexor-step-abc".to_string(),
            timeout_secs: 600,
        };
        let msg = err.to_string();
        assert!(msg.contains("nexor-step-abc"));
        assert!(msg.contains("600"));
        assert!(msg.contains("timed out"));
    }

    // ── build_create_args ────────────────────────────────────────────────

    #[test]
    fn build_create_args_includes_security_flags() {
        let config = ContainerConfig::default();
        let args = build_create_args("test-container", &config);
        assert!(
            args.contains(&"--cap-drop=ALL".to_string()),
            "Missing --cap-drop=ALL"
        );
        assert!(
            args.contains(&"--security-opt=no-new-privileges".to_string()),
            "Missing --security-opt=no-new-privileges"
        );
    }

    #[test]
    fn build_create_args_includes_resource_limits() {
        let config = ContainerConfig {
            memory_limit: "4g".to_string(),
            cpu_limit: "3.0".to_string(),
            ..ContainerConfig::default()
        };
        let args = build_create_args("test-container", &config);
        assert!(args.contains(&"--memory=4g".to_string()));
        assert!(args.contains(&"--cpus=3.0".to_string()));
    }

    #[test]
    fn build_create_args_includes_network_mode() {
        let config = ContainerConfig {
            network_mode: Some("container:vpn-sidecar-123".to_string()),
            ..ContainerConfig::default()
        };
        let args = build_create_args("test-container", &config);
        assert!(args.contains(&"--network=container:vpn-sidecar-123".to_string()));
    }

    #[test]
    fn build_create_args_no_network_without_config() {
        let config = ContainerConfig::default();
        let args = build_create_args("test-container", &config);
        assert!(
            !args.iter().any(|a| a.starts_with("--network=")),
            "Should not have --network when network_mode is None"
        );
    }

    #[test]
    fn build_create_args_includes_env_vars() {
        let config = ContainerConfig {
            env_vars: vec![
                ("FOO".to_string(), "bar".to_string()),
                ("BAZ".to_string(), "qux".to_string()),
            ],
            ..ContainerConfig::default()
        };
        let args = build_create_args("test-container", &config);
        assert!(args.contains(&"--env=FOO=bar".to_string()));
        assert!(args.contains(&"--env=BAZ=qux".to_string()));
    }

    #[test]
    fn build_create_args_ends_with_sleep_infinity() {
        let config = ContainerConfig::default();
        let args = build_create_args("test-container", &config);
        let len = args.len();
        assert!(len >= 2);
        assert_eq!(args[len - 2], "sleep");
        assert_eq!(args[len - 1], "infinity");
    }

    // ── semaphore ────────────────────────────────────────────────────────

    #[test]
    fn container_create_semaphore_is_accessible() {
        // Verify the lazy static initializes without panic
        let permits = CONTAINER_CREATE_SEMAPHORE.available_permits();
        assert_eq!(permits, crate::constants::CONTAINER_MAX_CONCURRENT_CREATES);
    }

    // ── spawn_reaper ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn spawn_reaper_shuts_down_on_token_cancel() {
        let token = tokio_util::sync::CancellationToken::new();
        let handle = ContainerManager::real().spawn_reaper(
            std::time::Duration::from_secs(3600),
            std::time::Duration::from_millis(50),
            token.clone(),
        );

        // Let it tick once
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(!handle.is_finished());

        // Cancel and verify it stops
        token.cancel();
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "Reaper should shut down within 2 seconds");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Mock-based tests for Docker-touching functions
    // ═══════════════════════════════════════════════════════════════════════

    use std::sync::Arc;

    fn success_output(stdout: &str) -> CommandOutput {
        CommandOutput {
            exit_code: 0,
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    fn failure_output(exit_code: i32, stderr: &str) -> CommandOutput {
        CommandOutput {
            exit_code,
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    fn make_handle(cli: Arc<dyn DockerCli>) -> ContainerHandle {
        ContainerHandle {
            container_id: "abc123".to_string(),
            container_name: "nexor-step-test".to_string(),
            workdir: "/workspace".to_string(),
            command_timeout_secs: 300,
            cli,
        }
    }

    // ── exec ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn exec_success() {
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(|_| Ok(success_output("hello world\n")));

        let handle = make_handle(Arc::new(mock));
        let result = handle.exec(&["echo", "hello world"]).await.unwrap();
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello world"));
        assert!(!result.truncated);
    }

    #[tokio::test]
    async fn exec_non_zero_exit() {
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(|_| Ok(failure_output(1, "command not found")));

        let handle = make_handle(Arc::new(mock));
        let result = handle.exec(&["bad-cmd"]).await.unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, 1);
        assert!(result.stderr.contains("command not found"));
    }

    #[tokio::test]
    async fn exec_output_truncated() {
        let mut mock = MockDockerCli::new();
        let big_output = vec![b'x'; crate::constants::CONTAINER_MAX_OUTPUT_BYTES + 1000];
        mock.expect_run().returning(move |_| {
            Ok(CommandOutput {
                exit_code: 0,
                stdout: big_output.clone(),
                stderr: Vec::new(),
            })
        });

        let handle = make_handle(Arc::new(mock));
        let result = handle.exec(&["cat", "bigfile"]).await.unwrap();
        assert!(result.truncated);
        assert!(result.stdout.contains("[truncated,"));
    }

    #[tokio::test]
    async fn exec_docker_spawn_failed() {
        let mut mock = MockDockerCli::new();
        mock.expect_run().returning(|_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "docker not found",
            ))
        });

        let handle = make_handle(Arc::new(mock));
        let result = handle.exec(&["ls"]).await;
        assert!(matches!(
            result.unwrap_err(),
            ContainerError::DockerSpawnFailed {
                operation: "exec",
                ..
            }
        ));
    }

    // ── write_file ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn write_file_simple_path() {
        let mut mock = MockDockerCli::new();
        // write_file for a root-level file only calls run_with_stdin (no mkdir needed)
        mock.expect_run_with_stdin()
            .returning(|_, _| Ok(success_output("")));
        // exec_shell for mkdir (parent is empty for root file, so no mkdir call)
        // Actually, "file.txt" has parent "" which is filtered out, so only run_with_stdin is called

        let handle = make_handle(Arc::new(mock));
        let result = handle.write_file("file.txt", "content").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn write_file_nested_path() {
        let mut mock = MockDockerCli::new();
        // First call: exec for mkdir -p (via exec_shell → exec → cli.run)
        // Second call: run_with_stdin for the actual write
        mock.expect_run()
            .times(1)
            .returning(|_| Ok(success_output("")));
        mock.expect_run_with_stdin()
            .times(1)
            .returning(|_, _| Ok(success_output("")));

        let handle = make_handle(Arc::new(mock));
        let result = handle.write_file("src/lib.rs", "fn main() {}").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn write_file_cat_fails() {
        let mut mock = MockDockerCli::new();
        mock.expect_run_with_stdin()
            .returning(|_, _| Ok(failure_output(1, "permission denied")));

        let handle = make_handle(Arc::new(mock));
        let result = handle.write_file("file.txt", "content").await;
        assert!(matches!(
            result.unwrap_err(),
            ContainerError::CommandFailed { exit_code: 1, .. }
        ));
    }

    // ── is_alive ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn is_alive_true() {
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(|_| Ok(success_output("true\n")));

        let handle = make_handle(Arc::new(mock));
        assert!(handle.is_alive().await);
    }

    #[tokio::test]
    async fn is_alive_false() {
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(|_| Ok(success_output("false\n")));

        let handle = make_handle(Arc::new(mock));
        assert!(!handle.is_alive().await);
    }

    // ── read_file ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn read_file_success() {
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(|_| Ok(success_output("file contents here")));

        let handle = make_handle(Arc::new(mock));
        let content = handle.read_file("src/main.rs").await.unwrap();
        assert_eq!(content, "file contents here");
    }

    // ── list_files ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_files_returns_a_tree_with_directories_marked() {
        // `find` prints paths under the start directory; the shell loop adds the
        // trailing slash. Both are stripped back to paths relative to `path`.
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(|_| Ok(success_output("./README.md\n./src/\n./src/main.rs\n")));

        let handle = make_handle(Arc::new(mock));
        let (files, dropped) = handle.list_files(".", 3).await.unwrap();
        assert_eq!(files, vec!["README.md", "src/", "src/main.rs"]);
        assert_eq!(dropped, 0);
    }

    #[tokio::test]
    async fn list_files_caps_entries_and_reports_the_remainder() {
        // A silently truncated listing reads as a complete one, so the count of
        // what was dropped has to come back with it.
        let over = LIST_FILES_MAX_ENTRIES + 25;
        let stdout: String = (0..over).map(|i| format!("./f{i}\n")).collect();
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(move |_| Ok(success_output(&stdout)));

        let handle = make_handle(Arc::new(mock));
        let (files, dropped) = handle.list_files(".", 3).await.unwrap();
        assert_eq!(files.len(), LIST_FILES_MAX_ENTRIES);
        assert_eq!(dropped, 25);
    }

    #[tokio::test]
    async fn list_files_clamps_depth_to_the_maximum() {
        let mut mock = MockDockerCli::new();
        mock.expect_run().returning(|args| {
            let joined = args.join(" ");
            assert!(
                joined.contains(&format!("-maxdepth {LIST_FILES_MAX_DEPTH}")),
                "{joined}"
            );
            Ok(success_output(""))
        });

        let handle = make_handle(Arc::new(mock));
        let (files, _) = handle.list_files(".", 99).await.unwrap();
        assert!(files.is_empty());
    }

    // ── git ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn git_success() {
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(|_| Ok(success_output("abc1234 Initial commit\n")));

        let handle = make_handle(Arc::new(mock));
        let output = handle.git(&["log", "--oneline", "-1"]).await.unwrap();
        assert!(output.contains("abc1234"));
    }

    // ── destroy_container ───────────────────────────────────────────────

    #[tokio::test]
    async fn destroy_rm_succeeds() {
        let mut mock = MockDockerCli::new();
        mock.expect_run().times(1).returning(|args| {
            assert!(args.iter().any(|a| a == "rm"));
            assert!(args.iter().any(|a| a == "-f"));
            Ok(success_output(""))
        });

        let handle = make_handle(Arc::new(mock));
        let result = ContainerManager::destroy_container(&handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn destroy_rm_spawn_fails() {
        let mut mock = MockDockerCli::new();
        mock.expect_run().times(1).returning(|_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "docker not found",
            ))
        });

        let handle = make_handle(Arc::new(mock));
        let result = ContainerManager::destroy_container(&handle).await;
        assert!(matches!(
            result.unwrap_err(),
            ContainerError::DockerSpawnFailed {
                operation: "rm",
                ..
            }
        ));
    }

    // ── disconnect_bridge_network ───────────────────────────────────────

    #[tokio::test]
    async fn disconnect_succeeds() {
        let mut mock = MockDockerCli::new();
        mock.expect_run().returning(|_| Ok(success_output("")));

        let result = disconnect_bridge_network(&mock, "abc123", "nexor-step-test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn disconnect_already_disconnected() {
        let mut mock = MockDockerCli::new();
        mock.expect_run().returning(|_| {
            Ok(failure_output(
                1,
                "Error response from daemon: container abc123 is not connected to the network bridge",
            ))
        });

        let result = disconnect_bridge_network(&mock, "abc123", "nexor-step-test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn disconnect_other_error() {
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(|_| Ok(failure_output(1, "some other error")));

        let result = disconnect_bridge_network(&mock, "abc123", "nexor-step-test").await;
        assert!(matches!(
            result.unwrap_err(),
            ContainerError::NetworkDisconnectFailed { .. }
        ));
    }

    // ── reap_orphaned_containers ────────────────────────────────────────

    #[tokio::test]
    async fn reap_no_containers() {
        let mut mock = MockDockerCli::new();
        mock.expect_run().returning(|_| Ok(success_output("")));

        let mgr = ContainerManager::new(Arc::new(mock));
        let reaped = mgr
            .reap_orphaned_containers(std::time::Duration::from_secs(3600))
            .await;
        assert_eq!(reaped, 0);
    }

    #[tokio::test]
    async fn reap_all_old() {
        let mut mock = MockDockerCli::new();
        // ps returns two old containers
        mock.expect_run().times(1).returning(|args| {
            if args.iter().any(|a| a == "ps") {
                Ok(success_output(
                    "id1\t2020-01-01 00:00:00 +0000 UTC\tnexor-step-old1\n\
                         id2\t2020-01-01 00:00:00 +0000 UTC\tnexor-step-old2\n",
                ))
            } else {
                Ok(success_output(""))
            }
        });
        // Two rm calls
        mock.expect_run().times(2).returning(|args| {
            assert!(args.iter().any(|a| a == "rm"));
            Ok(success_output(""))
        });

        let mgr = ContainerManager::new(Arc::new(mock));
        let reaped = mgr
            .reap_orphaned_containers(std::time::Duration::from_secs(3600))
            .await;
        assert_eq!(reaped, 2);
    }

    #[tokio::test]
    async fn reap_docker_ps_fails() {
        let mut mock = MockDockerCli::new();
        mock.expect_run().returning(|_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "docker not available",
            ))
        });

        let mgr = ContainerManager::new(Arc::new(mock));
        let reaped = mgr
            .reap_orphaned_containers(std::time::Duration::from_secs(3600))
            .await;
        assert_eq!(reaped, 0);
    }

    #[tokio::test]
    async fn reap_malformed_lines() {
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(|_| Ok(success_output("malformed line without tabs\nanother bad\n")));

        let mgr = ContainerManager::new(Arc::new(mock));
        let reaped = mgr
            .reap_orphaned_containers(std::time::Duration::from_secs(3600))
            .await;
        assert_eq!(reaped, 0);
    }

    // ── create_container ────────────────────────────────────────────────

    #[tokio::test]
    async fn create_happy_path() {
        let mut mock = MockDockerCli::new();
        let call_count = std::sync::atomic::AtomicU32::new(0);
        // The sequence of calls:
        // 1. docker create → returns container id
        // 2. docker start → success
        // 3. docker exec (git clone) → success
        // 4. docker exec (git config user.email) → success
        // 5. docker exec (git config user.name) → success
        // 6. docker network disconnect → success
        mock.expect_run().returning(move |args| {
            let n = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match n {
                0 => {
                    // create
                    assert!(args.iter().any(|a| a == "create"));
                    Ok(success_output("container-id-123\n"))
                }
                1 => {
                    // start
                    assert!(args.iter().any(|a| a == "start"));
                    Ok(success_output(""))
                }
                2 => {
                    // git clone (via exec)
                    Ok(success_output("Cloning into '.'..."))
                }
                3 | 4 => {
                    // git config
                    Ok(success_output(""))
                }
                5 => {
                    // network disconnect
                    Ok(success_output(""))
                }
                _ => Ok(success_output("")),
            }
        });

        let config = ContainerConfig {
            clone_url: "https://github.com/owner/repo.git".to_string(),
            github_token: RedactedString::new("ghp_test"),
            ..ContainerConfig::default()
        };
        let mgr = ContainerManager::new(Arc::new(mock));
        let handle = mgr.create_container(&config).await.unwrap();
        assert!(handle.container_name().starts_with("nexor-step-"));
    }

    #[tokio::test]
    async fn create_docker_create_fails() {
        let mut mock = MockDockerCli::new();
        mock.expect_run()
            .returning(|_| Ok(failure_output(1, "no such image")));

        let config = ContainerConfig::default();
        let mgr = ContainerManager::new(Arc::new(mock));
        let result = mgr.create_container(&config).await;
        assert!(matches!(
            result.unwrap_err(),
            ContainerError::CreationFailed(_)
        ));
    }

    #[tokio::test]
    async fn create_docker_start_fails() {
        let mut mock = MockDockerCli::new();
        let call_count = std::sync::atomic::AtomicU32::new(0);
        mock.expect_run().returning(move |_| {
            let n = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match n {
                0 => Ok(success_output("container-id-123\n")), // create
                1 => Ok(failure_output(1, "start failed")),    // start fails
                2 => Ok(success_output("")),                   // rm cleanup
                _ => Ok(success_output("")),
            }
        });

        let config = ContainerConfig::default();
        let mgr = ContainerManager::new(Arc::new(mock));
        let result = mgr.create_container(&config).await;
        assert!(matches!(
            result.unwrap_err(),
            ContainerError::CreationFailed(_)
        ));
    }

    #[tokio::test]
    async fn create_clone_exit_nonzero() {
        let mut mock = MockDockerCli::new();
        let call_count = std::sync::atomic::AtomicU32::new(0);
        mock.expect_run().returning(move |_| {
            let n = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match n {
                0 => Ok(success_output("container-id-123\n")), // create
                1 => Ok(success_output("")),                    // start
                2 => Ok(failure_output(128, "Authentication failed for 'https://x-access-token:ghp_secret@github.com/repo.git'")), // clone fails
                3 | 4 => Ok(success_output("")),                // destroy (stop + rm)
                _ => Ok(success_output("")),
            }
        });

        let config = ContainerConfig {
            clone_url: "https://github.com/owner/repo.git".to_string(),
            github_token: RedactedString::new("ghp_secret"),
            ..ContainerConfig::default()
        };
        let mgr = ContainerManager::new(Arc::new(mock));
        let result = mgr.create_container(&config).await;
        match result.unwrap_err() {
            ContainerError::CloneFailed { stderr } => {
                // Token should be sanitized
                assert!(!stderr.contains("ghp_secret"));
                assert!(stderr.contains("[REDACTED]"));
            }
            other => panic!("Expected CloneFailed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn create_no_branch_skips_checkout() {
        let mut mock = MockDockerCli::new();
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let calls_clone = calls.clone();
        mock.expect_run().returning(move |args| {
            calls_clone.lock().unwrap().push(args.clone());
            let is_create = args.iter().any(|a| a == "create");
            if is_create {
                Ok(success_output("container-id\n"))
            } else {
                Ok(success_output(""))
            }
        });

        let config = ContainerConfig {
            clone_url: "https://github.com/owner/repo.git".to_string(),
            github_token: RedactedString::new("token"),
            branch: None, // No branch
            ..ContainerConfig::default()
        };
        let mgr = ContainerManager::new(Arc::new(mock));
        let _ = mgr.create_container(&config).await;

        // Verify no checkout command was issued
        let calls = calls.lock().unwrap();
        let has_checkout = calls
            .iter()
            .any(|args| args.iter().any(|a| a.contains("checkout")));
        assert!(
            !has_checkout,
            "Should not have issued a git checkout when branch is None"
        );
    }

    // ── workspace volume: build_create_args ─────────────────────────────

    fn test_workspace_volume() -> WorkspaceVolume {
        WorkspaceVolume {
            volume_name: "nexor-jfs-workspace".to_string(),
            subpath: "workflows/abc/runs/def".to_string(),
        }
    }

    #[test]
    fn build_create_args_with_workspace_volume_includes_mount() {
        let config = ContainerConfig {
            workspace_volume: Some(test_workspace_volume()),
            ..ContainerConfig::default()
        };
        let args = build_create_args("test-container", &config);
        assert!(args.contains(&"--mount".to_string()));
        let mount_arg = args.iter().find(|a| a.contains("type=volume")).unwrap();
        assert!(
            mount_arg.contains("source=nexor-jfs-workspace"),
            "Should reference the named volume"
        );
        assert!(
            mount_arg.contains("target=/workspace"),
            "Should mount at /workspace"
        );
        assert!(
            mount_arg.contains("volume-subpath=workflows/abc/runs/def"),
            "Should include the subpath"
        );
    }

    #[test]
    fn build_create_args_with_workspace_volume_injects_env_vars() {
        let config = ContainerConfig {
            workspace_volume: Some(test_workspace_volume()),
            ..ContainerConfig::default()
        };
        let args = build_create_args("test-container", &config);
        assert!(
            !args.iter().any(|a| a.contains("VIRTUAL_ENV")),
            "Should not inject VIRTUAL_ENV (no venv in workspace)"
        );
        assert!(
            args.contains(&"--env=PIP_CACHE_DIR=/tmp/pip-cache".to_string()),
            "Should inject PIP_CACHE_DIR (stays in /tmp)"
        );
        assert!(
            args.contains(&"--env=PYTHONDONTWRITEBYTECODE=1".to_string()),
            "Should inject PYTHONDONTWRITEBYTECODE"
        );
        assert!(
            args.contains(&"--env=npm_config_cache=/tmp/npm-cache".to_string()),
            "Should inject npm_config_cache (stays in /tmp)"
        );
        assert!(
            args.contains(&"--env=CARGO_HOME=/workspace/.cargo".to_string()),
            "Should inject CARGO_HOME into workspace"
        );
        assert!(
            args.contains(&"--env=XDG_CACHE_HOME=/tmp/cache".to_string()),
            "Should inject XDG_CACHE_HOME (stays in /tmp)"
        );
    }

    #[test]
    fn build_create_args_without_workspace_volume_no_mount() {
        let config = ContainerConfig::default();
        let args = build_create_args("test-container", &config);
        assert!(
            !args.contains(&"--mount".to_string()),
            "Should not have --mount when workspace_volume is None"
        );
        assert!(
            !args.iter().any(|a| a.contains("VIRTUAL_ENV")),
            "Should not inject workspace env vars when no volume"
        );
    }

    // ── workspace volume + overlay: build_create_args ───────────────────

    #[test]
    fn build_create_args_overlay_mode_mounts_at_workspace_base() {
        let config = ContainerConfig {
            workspace_volume: Some(test_workspace_volume()),
            overlay_enabled: true,
            ..ContainerConfig::default()
        };
        let args = build_create_args("test-container", &config);
        let mount_arg = args.iter().find(|a| a.contains("type=volume")).unwrap();
        assert!(
            mount_arg.contains("target=/workspace-base"),
            "Overlay mode should mount at /workspace-base, got: {}",
            mount_arg
        );
        assert!(
            !mount_arg.contains(",readonly"),
            "Overlay mode should NOT mount read-only (breaks overlay mount), got: {}",
            mount_arg
        );
    }

    #[test]
    fn build_create_args_overlay_mode_adds_sys_admin() {
        let config = ContainerConfig {
            workspace_volume: Some(test_workspace_volume()),
            overlay_enabled: true,
            ..ContainerConfig::default()
        };
        let args = build_create_args("test-container", &config);
        assert!(
            args.contains(&"--cap-add=SYS_ADMIN".to_string()),
            "Overlay mode should add SYS_ADMIN capability"
        );
        // cap-drop=ALL should still be present (applied earlier in args)
        assert!(
            args.contains(&"--cap-drop=ALL".to_string()),
            "Should still have --cap-drop=ALL"
        );
    }

    #[test]
    fn build_create_args_non_overlay_no_sys_admin() {
        let config = ContainerConfig {
            workspace_volume: Some(test_workspace_volume()),
            overlay_enabled: false,
            ..ContainerConfig::default()
        };
        let args = build_create_args("test-container", &config);
        assert!(
            !args.contains(&"--cap-add=SYS_ADMIN".to_string()),
            "Non-overlay mode should not add SYS_ADMIN"
        );
        let mount_arg = args.iter().find(|a| a.contains("type=volume")).unwrap();
        assert!(
            mount_arg.contains("target=/workspace"),
            "Non-overlay mode should mount at /workspace"
        );
        assert!(
            !mount_arg.contains("readonly"),
            "Non-overlay mode should not be read-only"
        );
    }

    #[test]
    fn build_create_args_overlay_without_workspace_volume_no_effect() {
        let config = ContainerConfig {
            overlay_enabled: true,
            workspace_volume: None,
            ..ContainerConfig::default()
        };
        let args = build_create_args("test-container", &config);
        assert!(
            !args.contains(&"--cap-add=SYS_ADMIN".to_string()),
            "Overlay without workspace_volume should have no effect"
        );
        assert!(
            !args.contains(&"--mount".to_string()),
            "Should not have --mount without workspace_volume"
        );
    }

    #[test]
    fn build_create_args_overlay_still_injects_env_vars() {
        let config = ContainerConfig {
            workspace_volume: Some(test_workspace_volume()),
            overlay_enabled: true,
            ..ContainerConfig::default()
        };
        let args = build_create_args("test-container", &config);
        assert!(
            !args.iter().any(|a| a.contains("VIRTUAL_ENV")),
            "Should not inject VIRTUAL_ENV (no venv in workspace)"
        );
        assert!(
            args.contains(&"--env=PIP_CACHE_DIR=/tmp/pip-cache".to_string()),
            "Overlay mode should still inject workspace env vars"
        );
    }

    // ── workspace volume: create_container (skips git clone) ──────────

    #[tokio::test]
    async fn create_with_workspace_volume_skips_git_clone() {
        let mut mock = MockDockerCli::new();
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let calls_clone = calls.clone();
        mock.expect_run().returning(move |args| {
            calls_clone.lock().unwrap().push(args.clone());
            let is_create = args.iter().any(|a| a == "create");
            if is_create {
                Ok(success_output("container-id\n"))
            } else {
                Ok(success_output(""))
            }
        });

        let config = ContainerConfig {
            workspace_volume: Some(test_workspace_volume()),
            ..ContainerConfig::default()
        };
        let mgr = ContainerManager::new(Arc::new(mock));
        let handle = mgr.create_container(&config).await.unwrap();
        assert!(handle.container_name().starts_with("nexor-step-"));

        // Should only have create + start (no clone, no config, no disconnect)
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            2,
            "Workspace mode should only have create + start, got {} calls",
            calls.len()
        );
        assert!(
            calls[0].iter().any(|a| a == "create"),
            "First call should be create"
        );
        assert!(
            calls[1].iter().any(|a| a == "start"),
            "Second call should be start"
        );
    }

    #[tokio::test]
    async fn create_network_mode_skips_disconnect() {
        let mut mock = MockDockerCli::new();
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let calls_clone = calls.clone();
        mock.expect_run().returning(move |args| {
            calls_clone.lock().unwrap().push(args.clone());
            let is_create = args.iter().any(|a| a == "create");
            if is_create {
                Ok(success_output("container-id\n"))
            } else {
                Ok(success_output(""))
            }
        });

        let config = ContainerConfig {
            clone_url: "https://github.com/owner/repo.git".to_string(),
            github_token: RedactedString::new("token"),
            network_mode: Some("container:vpn-sidecar".to_string()),
            ..ContainerConfig::default()
        };
        let mgr = ContainerManager::new(Arc::new(mock));
        let _ = mgr.create_container(&config).await;

        // Should not call network disconnect when network_mode is Some
        let calls = calls.lock().unwrap();
        let has_disconnect = calls
            .iter()
            .any(|args| args.iter().any(|a| a == "disconnect"));
        assert!(
            !has_disconnect,
            "Should not disconnect network when network_mode is set"
        );
    }
}
