#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn vpn_sidecar_handle_clone() {
        let handle = VpnSidecarHandle {
            container_id: "abc123".to_string(),
            container_name: "nexor-vpn-test".to_string(),
            peer_id: "peer-1".to_string(),
        };
        let cloned = handle.clone();
        assert_eq!(cloned.container_id, "abc123");
        assert_eq!(cloned.container_name, "nexor-vpn-test");
        assert_eq!(cloned.peer_id, "peer-1");
    }

    #[test]
    fn vpn_sidecar_handle_debug() {
        let handle = VpnSidecarHandle {
            container_id: "abc123".to_string(),
            container_name: "nexor-vpn-test".to_string(),
            peer_id: "peer-1".to_string(),
        };
        let debug_str = format!("{:?}", handle);
        assert!(debug_str.contains("abc123"));
        assert!(debug_str.contains("nexor-vpn-test"));
        assert!(debug_str.contains("peer-1"));
    }

    #[test]
    fn sidecar_name_format() {
        let name = format!(
            "{}-{}",
            crate::constants::VPN_SIDECAR_NAME_PREFIX,
            uuid::Uuid::new_v4()
        );
        assert!(name.starts_with("nexor-vpn-"));
    }

    #[test]
    fn reaper_uses_correct_name_prefix() {
        // The reaper filters by name prefix — verify the constant matches
        // what create_sidecar uses in container names.
        let prefix = crate::constants::VPN_SIDECAR_NAME_PREFIX;
        assert_eq!(prefix, "nexor-vpn");
        // Container names are formatted as "{prefix}-{uuid}"
        let sample = format!("{}-{}", prefix, uuid::Uuid::new_v4());
        assert!(sample.starts_with(prefix));
    }
}
