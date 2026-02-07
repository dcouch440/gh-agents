#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::super::super::super::VpnError;
    use super::super::{is_retryable, is_server_error, vpn_backoff_config, vpn_with_retry};

    // ── is_retryable ──────────────────────────────────────────────────────

    #[test]
    fn retryable_api_unreachable() {
        let err = VpnError::ApiUnreachable("connection refused".into());
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_peer_creation_5xx() {
        let err = VpnError::PeerCreationFailed {
            reason: "HTTP 500: internal server error".into(),
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_peer_deletion_503() {
        let err = VpnError::PeerDeletionFailed {
            peer_id: "abc".into(),
            reason: "HTTP 503: service unavailable".into(),
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn retryable_config_retrieval_502() {
        let err = VpnError::ConfigRetrievalFailed {
            peer_id: "abc".into(),
            reason: "HTTP 502: bad gateway".into(),
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn not_retryable_auth_failed() {
        assert!(!is_retryable(&VpnError::AuthFailed));
    }

    #[test]
    fn not_retryable_sidecar_failed() {
        let err = VpnError::SidecarFailed("docker create failed".into());
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_health_check_timeout() {
        let err = VpnError::HealthCheckTimeout { timeout_secs: 30 };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_io_error() {
        let err = VpnError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_peer_creation_4xx() {
        let err = VpnError::PeerCreationFailed {
            reason: "HTTP 400: bad request".into(),
        };
        assert!(!is_retryable(&err));
    }

    #[test]
    fn not_retryable_config_retrieval_404() {
        let err = VpnError::ConfigRetrievalFailed {
            peer_id: "abc".into(),
            reason: "HTTP 404: not found".into(),
        };
        assert!(!is_retryable(&err));
    }

    // ── is_server_error ───────────────────────────────────────────────────

    #[test]
    fn server_error_5xx() {
        assert!(is_server_error("HTTP 500: internal"));
        assert!(is_server_error("HTTP 502: bad gateway"));
        assert!(is_server_error("HTTP 503: unavailable"));
    }

    #[test]
    fn not_server_error_4xx() {
        assert!(!is_server_error("HTTP 400: bad request"));
        assert!(!is_server_error("HTTP 404: not found"));
        assert!(!is_server_error("HTTP 422: unprocessable"));
    }

    #[test]
    fn not_server_error_other() {
        assert!(!is_server_error("invalid response: eof"));
        assert!(!is_server_error(""));
    }

    // ── vpn_backoff_config ────────────────────────────────────────────────

    #[test]
    fn backoff_config_matches_constants() {
        let config = vpn_backoff_config();
        assert_eq!(
            config.initial_delay.as_millis() as u64,
            crate::constants::VPN_RETRY_INITIAL_BACKOFF_MS
        );
        assert_eq!(
            config.max_delay.as_secs(),
            crate::constants::VPN_RETRY_MAX_BACKOFF_SECS
        );
        assert_eq!(config.max_retries, crate::constants::VPN_RETRY_MAX_ATTEMPTS);
    }

    // ── vpn_with_retry ────────────────────────────────────────────────────

    #[tokio::test]
    async fn retry_succeeds_first_attempt() {
        let result = vpn_with_retry(|| async { Ok::<_, VpnError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn retry_succeeds_after_transient_failure() {
        let attempts = AtomicU32::new(0);
        let result = vpn_with_retry(|| {
            let n = attempts.fetch_add(1, Ordering::SeqCst);
            async move {
                if n == 0 {
                    Err(VpnError::ApiUnreachable("connection refused".into()))
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
        let result = vpn_with_retry(|| {
            attempts.fetch_add(1, Ordering::SeqCst);
            async { Err::<i32, _>(VpnError::AuthFailed) }
        })
        .await;
        assert!(result.is_err());
        // Should not retry — only 1 attempt
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retry_exhausts_retries() {
        let attempts = AtomicU32::new(0);
        let result = vpn_with_retry(|| {
            attempts.fetch_add(1, Ordering::SeqCst);
            async { Err::<i32, _>(VpnError::ApiUnreachable("refused".into())) }
        })
        .await;
        assert!(result.is_err());
        // 1 initial + 3 retries = 4 total
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            1 + crate::constants::VPN_RETRY_MAX_ATTEMPTS
        );
    }
}
