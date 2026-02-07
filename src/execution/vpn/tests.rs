#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn vpn_error_display_api_unreachable() {
        let err = VpnError::ApiUnreachable("connection refused".to_string());
        assert_eq!(
            err.to_string(),
            "wg-easy API unreachable: connection refused"
        );
    }

    #[test]
    fn vpn_error_display_auth_failed() {
        let err = VpnError::AuthFailed;
        assert_eq!(err.to_string(), "wg-easy authentication failed");
    }

    #[test]
    fn vpn_error_display_peer_creation_failed() {
        let err = VpnError::PeerCreationFailed {
            reason: "quota exceeded".to_string(),
        };
        assert_eq!(err.to_string(), "peer creation failed: quota exceeded");
    }

    #[test]
    fn vpn_error_display_peer_deletion_failed() {
        let err = VpnError::PeerDeletionFailed {
            peer_id: "abc123".to_string(),
            reason: "not found".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "peer deletion failed for abc123: not found"
        );
    }

    #[test]
    fn vpn_error_display_config_retrieval_failed() {
        let err = VpnError::ConfigRetrievalFailed {
            peer_id: "abc123".to_string(),
            reason: "HTTP 404".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "peer config retrieval failed for abc123: HTTP 404"
        );
    }

    #[test]
    fn vpn_error_display_health_check_timeout() {
        let err = VpnError::HealthCheckTimeout { timeout_secs: 30 };
        assert_eq!(err.to_string(), "VPN health check failed after 30s");
    }

    #[test]
    fn vpn_error_display_sidecar_failed() {
        let err = VpnError::SidecarFailed("container exited".to_string());
        assert_eq!(
            err.to_string(),
            "VPN sidecar container failed: container exited"
        );
    }

    #[test]
    fn wg_easy_config_direct_construction() {
        let config = WgEasyConfig {
            base_url: "http://wg.example.com:51821".to_string(),
            password: RedactedString::new("secret123"),
            timeout_secs: 15,
        };
        assert_eq!(config.base_url, "http://wg.example.com:51821");
        assert_eq!(config.password.expose(), "secret123");
        assert_eq!(config.timeout_secs, 15);
    }

    #[test]
    fn wg_easy_config_password_is_redacted_in_debug() {
        let config = WgEasyConfig {
            base_url: "http://localhost:51821".to_string(),
            password: RedactedString::new("super-secret"),
            timeout_secs: 10,
        };
        let debug_str = format!("{:?}", config);
        assert!(!debug_str.contains("super-secret"));
        assert!(debug_str.contains("[REDACTED]"));
    }

    #[test]
    fn wg_easy_client_can_be_constructed() {
        let config = WgEasyConfig {
            base_url: "http://localhost:51821".to_string(),
            password: RedactedString::new("test"),
            timeout_secs: 5,
        };
        let _client = WgEasyClient::new(config);
    }

    // ── Config validation ─────────────────────────────────────────────────

    #[test]
    fn validate_wg_config_full_tunnel_with_dns() {
        let config = "[Interface]\nPrivateKey = abc123\nAddress = 10.8.0.2/32\nDNS = 10.8.0.1\n\n[Peer]\nPublicKey = xyz789\nAllowedIPs = 0.0.0.0/0\nEndpoint = vpn.example.com:51820\n";
        assert!(super::super::validate_wg_config(config).is_ok());
    }

    #[test]
    fn validate_wg_config_rejects_split_tunnel() {
        let config = "[Interface]\nPrivateKey = abc123\nAddress = 10.8.0.2/32\nDNS = 10.8.0.1\n\n[Peer]\nPublicKey = xyz789\nAllowedIPs = 10.0.0.0/8\nEndpoint = vpn.example.com:51820\n";
        let err = super::super::validate_wg_config(config).unwrap_err();
        assert!(err.to_string().contains("AllowedIPs"));
    }

    #[test]
    fn validate_wg_config_rejects_missing_dns() {
        let config = "[Interface]\nPrivateKey = abc123\nAddress = 10.8.0.2/32\n\n[Peer]\nPublicKey = xyz789\nAllowedIPs = 0.0.0.0/0\nEndpoint = vpn.example.com:51820\n";
        let err = super::super::validate_wg_config(config).unwrap_err();
        assert!(err.to_string().contains("DNS"));
    }

    #[test]
    fn validate_wg_config_accepts_dual_stack() {
        let config = "[Interface]\nPrivateKey = abc123\nAddress = 10.8.0.2/32\nDNS = 10.8.0.1\n\n[Peer]\nPublicKey = xyz789\nAllowedIPs = 0.0.0.0/0, ::/0\nEndpoint = vpn.example.com:51820\n";
        assert!(super::super::validate_wg_config(config).is_ok());
    }

    #[test]
    fn validate_wg_config_rejects_empty() {
        let err = super::super::validate_wg_config("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn validate_wg_config_whitespace_tolerance() {
        let config = "[Interface]\nPrivateKey = abc123\nAddress = 10.8.0.2/32\nDNS = 10.8.0.1\n\n[Peer]\nPublicKey = xyz789\n  AllowedIPs = 0.0.0.0/0  \nEndpoint = vpn.example.com:51820\n";
        assert!(super::super::validate_wg_config(config).is_ok());
    }

    #[test]
    fn vpn_error_display_config_validation_failed() {
        let err = VpnError::ConfigValidationFailed {
            reason: "missing DNS".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "WireGuard config validation failed: missing DNS"
        );
    }

    #[test]
    fn wg_easy_client_starts_unauthenticated() {
        let config = WgEasyConfig {
            base_url: "http://localhost:51821".to_string(),
            password: RedactedString::new("test"),
            timeout_secs: 5,
        };
        let client = WgEasyClient::new(config);
        assert!(!client.is_authenticated());
    }
}
