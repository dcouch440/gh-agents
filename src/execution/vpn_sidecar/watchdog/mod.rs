//! VPN tunnel watchdog — monitors WireGuard health during agent execution.
//!
//! Runs alongside the execution engine via `tokio::select!`. If the tunnel
//! drops, the watchdog returns an error which causes the select to cancel
//! the execution future, providing fast failure instead of a 300s hang.

use std::process::Stdio;

use tokio::process::Command;
use tracing::{debug, warn};

use super::super::vpn::VpnError;

#[cfg(test)]
mod tests;

/// Monitor a VPN sidecar's WireGuard tunnel, returning only when the tunnel
/// is considered dead.
///
/// Designed to be raced against execution via `tokio::select!` — when execution
/// completes first, this future is simply dropped (cancelled). If the tunnel
/// fails first, the returned error causes the select to abort execution.
///
/// Returns `VpnError` directly (not `Result`) because this function only
/// returns when something goes wrong. "Everything is fine" means keep running
/// forever (until dropped).
pub async fn monitor_vpn_tunnel(
    container_id: &str,
    interval_secs: u64,
    max_failures: u32,
) -> VpnError {
    let mut consecutive_failures: u32 = 0;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;

        if check_tunnel_health(container_id).await {
            if consecutive_failures > 0 {
                debug!(
                    container_id,
                    previous_failures = consecutive_failures,
                    "VPN watchdog: tunnel recovered"
                );
            }
            consecutive_failures = 0;
        } else {
            consecutive_failures += 1;
            warn!(
                container_id,
                consecutive_failures, max_failures, "VPN watchdog: health check failed"
            );

            if consecutive_failures >= max_failures {
                warn!(
                    container_id,
                    consecutive_failures,
                    "VPN watchdog: tunnel considered dead, aborting execution"
                );
                return VpnError::HealthCheckTimeout {
                    timeout_secs: interval_secs * u64::from(max_failures),
                };
            }
        }
    }
}

/// Single health check: `wg show wg0` then fallback to ping.
///
/// Returns `true` if the tunnel appears healthy.
async fn check_tunnel_health(container_id: &str) -> bool {
    // Phase 1: Check WireGuard interface status
    let wg_output = Command::new("docker")
        .args(["exec", container_id, "wg", "show", "wg0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match wg_output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.contains("peer:") || stdout.contains("endpoint:") {
                debug!(container_id, "VPN watchdog: tunnel healthy");
                return true;
            }
        }
        _ => {}
    }

    // Phase 2: Fallback — ping the WireGuard gateway
    let ping_output = Command::new("docker")
        .args([
            "exec",
            container_id,
            "ping",
            "-c",
            "1",
            "-W",
            "2",
            crate::constants::VPN_HEALTH_CHECK_GATEWAY,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    matches!(ping_output, Ok(ref out) if out.status.success())
}
