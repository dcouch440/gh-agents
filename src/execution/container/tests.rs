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
        let handle = ContainerManager::spawn_reaper(
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
}
