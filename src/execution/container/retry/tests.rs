#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::super::super::ContainerError;
    use super::super::{
        container_backoff_config, container_with_retry, is_retryable, is_transient_docker_error,
    };

    // ── is_retryable ──────────────────────────────────────────────────────

    #[test]
    fn retryable_docker_not_available() {
        let err = ContainerError::DockerNotAvailable("connection refused".into());
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_docker_spawn_failed() {
        let err = ContainerError::DockerSpawnFailed {
            operation: "create",
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "docker not found"),
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_create_timeout() {
        let err = ContainerError::CreateTimeout {
            container: "nexor-step-abc".into(),
            timeout_secs: 600,
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_creation_failed_daemon() {
        let err = ContainerError::CreationFailed(
            "Cannot connect to the Docker daemon at unix:///var/run/docker.sock".into(),
        );
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_creation_failed_503() {
        let err = ContainerError::CreationFailed("503 Service Unavailable".into());
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_creation_failed_connection_refused() {
        let err = ContainerError::CreationFailed("connection refused".into());
        assert!(is_retryable(&err));
    }

    #[test]
    fn not_retryable_creation_failed_generic() {
        let err = ContainerError::CreationFailed("Conflict: name already in use".into());
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_clone_failed() {
        let err = ContainerError::CloneFailed {
            stderr: "authentication failed".into(),
        };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_command_failed() {
        let err = ContainerError::CommandFailed {
            container: "test".into(),
            exit_code: 1,
            stderr: "error".into(),
        };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_timeout() {
        let err = ContainerError::Timeout {
            container: "test".into(),
            timeout_secs: 300,
        };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_not_running() {
        let err = ContainerError::NotRunning {
            container: "test".into(),
        };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_path_not_allowed() {
        let err = ContainerError::PathNotAllowed {
            path: "/etc/shadow".into(),
            reason: "absolute paths are not allowed".into(),
        };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_network_disconnect_failed() {
        let err = ContainerError::NetworkDisconnectFailed {
            container: "test".into(),
            stderr: "error".into(),
        };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_io_error() {
        let err =
            ContainerError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
        assert!(!is_retryable(&err));
    }

    // ── is_transient_docker_error ─────────────────────────────────────────

    #[test]
    fn transient_daemon_message() {
        assert!(is_transient_docker_error(
            "Cannot connect to the Docker daemon"
        ));
    }

    #[test]
    fn transient_503_message() {
        assert!(is_transient_docker_error("503 Service Unavailable"));
    }

    #[test]
    fn transient_connection_refused() {
        assert!(is_transient_docker_error("connection refused"));
    }

    #[test]
    fn transient_timeout_message() {
        assert!(is_transient_docker_error("request timeout exceeded"));
    }

    #[test]
    fn not_transient_generic() {
        assert!(!is_transient_docker_error("Conflict: name already in use"));
        assert!(!is_transient_docker_error("no such image"));
        assert!(!is_transient_docker_error(""));
    }

    // ── container_backoff_config ──────────────────────────────────────────

    #[test]
    fn backoff_config_matches_constants() {
        let config = container_backoff_config();
        assert_eq!(
            config.initial_delay.as_millis() as u64,
            crate::constants::CONTAINER_RETRY_INITIAL_BACKOFF_MS
        );
        assert_eq!(
            config.max_delay.as_secs(),
            crate::constants::CONTAINER_RETRY_MAX_BACKOFF_SECS
        );
        assert_eq!(
            config.max_retries,
            crate::constants::CONTAINER_RETRY_MAX_ATTEMPTS
        );
    }

    // ── container_with_retry ──────────────────────────────────────────────

    #[tokio::test]
    async fn retry_succeeds_first_attempt() {
        let result = container_with_retry(|| async { Ok::<_, ContainerError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn retry_succeeds_after_transient_failure() {
        let attempts = AtomicU32::new(0);
        let result = container_with_retry(|| {
            let n = attempts.fetch_add(1, Ordering::SeqCst);
            async move {
                if n == 0 {
                    Err(ContainerError::DockerNotAvailable(
                        "connection refused".into(),
                    ))
                } else {
                    Ok(42)
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn retry_stops_on_permanent_error() {
        let attempts = AtomicU32::new(0);
        let result = container_with_retry(|| {
            attempts.fetch_add(1, Ordering::SeqCst);
            async {
                Err::<i32, _>(ContainerError::CloneFailed {
                    stderr: "auth failed".into(),
                })
            }
        })
        .await;
        assert!(result.is_err());
        // Should not retry — only 1 attempt
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retry_exhausts_retries() {
        let attempts = AtomicU32::new(0);
        let result = container_with_retry(|| {
            attempts.fetch_add(1, Ordering::SeqCst);
            async {
                Err::<i32, _>(ContainerError::DockerNotAvailable(
                    "connection refused".into(),
                ))
            }
        })
        .await;
        assert!(result.is_err());
        // 1 initial + N retries
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            1 + crate::constants::CONTAINER_RETRY_MAX_ATTEMPTS
        );
    }
}
