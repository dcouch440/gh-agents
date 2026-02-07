#[cfg(test)]
mod integration_tests {
    use super::super::*;

    fn test_config() -> WgEasyConfig {
        WgEasyConfig {
            base_url: std::env::var("WGEASY_API_URL")
                .unwrap_or_else(|_| "http://localhost:51821".to_string()),
            password: RedactedString::new(
                std::env::var("WGEASY_PASSWORD").unwrap_or_else(|_| "test".to_string()),
            ),
            timeout_secs: 10,
        }
    }

    #[tokio::test]
    #[ignore] // Requires running wg-easy: docker compose --profile vpn up
    async fn vpn_peer_lifecycle() {
        let client = WgEasyClient::new(test_config());

        // Health check
        client.health_check().await.expect("health check failed");

        // Create peer
        let peer = client
            .create_peer("integration-test-peer")
            .await
            .expect("create failed");
        assert!(!peer.id.is_empty());
        assert_eq!(peer.name, "integration-test-peer");

        // Get config
        let config = client
            .get_peer_config(&peer.id)
            .await
            .expect("get config failed");
        assert!(config.contains("[Interface]"));

        // List peers
        let peers = client.list_peers().await.expect("list peers failed");
        assert!(peers.iter().any(|p| p.id == peer.id));

        // Delete peer
        client.delete_peer(&peer.id).await.expect("delete failed");

        // Verify deleted
        let peers_after = client.list_peers().await.expect("list after delete failed");
        assert!(!peers_after.iter().any(|p| p.id == peer.id));
    }

    #[tokio::test]
    #[ignore] // Requires running wg-easy
    async fn vpn_retry_on_success() {
        use super::super::retry::vpn_with_retry;

        let client = WgEasyClient::new(test_config());
        let peer = vpn_with_retry(|| client.create_peer("retry-test-peer"))
            .await
            .expect("retry-wrapped create failed");
        assert!(!peer.id.is_empty());

        // Cleanup
        client.delete_peer(&peer.id).await.expect("cleanup failed");
    }

    #[tokio::test]
    #[ignore] // Requires running wg-easy
    async fn vpn_session_caching() {
        let client = WgEasyClient::new(test_config());

        // First call authenticates
        client.health_check().await.expect("first check failed");
        assert!(client.is_authenticated());

        // Second call reuses session
        client.health_check().await.expect("second check failed");
        assert!(client.is_authenticated());
    }
}
