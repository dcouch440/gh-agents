#[cfg(test)]
mod tests {
    #[test]
    fn watchdog_detection_time_under_command_timeout() {
        let detection_secs = crate::constants::VPN_WATCHDOG_INTERVAL_SECS
            * u64::from(crate::constants::VPN_WATCHDOG_MAX_FAILURES);
        assert!(
            detection_secs < crate::constants::CONTAINER_COMMAND_TIMEOUT_SECS,
            "watchdog detection time ({detection_secs}s) must be less than command timeout ({}s)",
            crate::constants::CONTAINER_COMMAND_TIMEOUT_SECS,
        );
    }

    #[test]
    fn watchdog_uses_standard_health_check_gateway() {
        // The watchdog's check_tunnel_health uses the same gateway constant
        // as the initial wait_for_vpn_health — verify it's set.
        assert!(
            !crate::constants::VPN_HEALTH_CHECK_GATEWAY.is_empty(),
            "VPN_HEALTH_CHECK_GATEWAY must not be empty"
        );
    }
}
