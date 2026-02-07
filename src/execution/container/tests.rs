#[cfg(test)]
mod tests {
    use super::super::*;

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

    #[test]
    fn container_exec_result_success_check() {
        let result = ContainerExecResult {
            success: true,
            exit_code: 0,
            stdout: "ok".to_string(),
            stderr: String::new(),
            duration_ms: 100,
        };
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn container_exec_result_failure_check() {
        let result = ContainerExecResult {
            success: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            duration_ms: 50,
        };
        assert!(!result.success);
        assert_eq!(result.exit_code, 1);
    }

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
}
