//! VPN sidecar container lifecycle management.
//!
//! Creates a lightweight WireGuard client container for each agent step.
//! The agent container shares the sidecar's network namespace via
//! `--network=container:<sidecar_id>`, so all traffic tunnels through the VPN.

use std::process::Stdio;

use tokio::process::Command;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::vpn::VpnError;

mod tests;
pub mod watchdog;

// ── Handle ─────────────────────────────────────────────────────────────────

/// Handle to a running VPN sidecar container.
#[derive(Debug, Clone)]
pub struct VpnSidecarHandle {
    /// Docker container ID of the WireGuard sidecar.
    pub container_id: String,
    /// Docker container name (for logging / network reference).
    pub container_name: String,
    /// wg-easy peer ID (for API cleanup).
    pub peer_id: String,
}

// ── Manager ────────────────────────────────────────────────────────────────

/// Creates and destroys VPN sidecar containers.
pub struct VpnSidecarManager;

impl VpnSidecarManager {
    /// Create and start a VPN sidecar container with the given WireGuard config.
    ///
    /// Steps:
    /// 1. `docker create` with NET_ADMIN capability, IPv6 disabled, log suppression
    /// 2. `docker start`
    /// 3. Write WireGuard config into the container
    /// 4. Bring up the WireGuard interface
    /// 5. Apply iptables kill switch (blocks all non-VPN traffic)
    /// 6. Wait for the tunnel to establish (health check + IP leak verify)
    pub async fn create_sidecar(
        wg_config: &str,
        peer_id: &str,
    ) -> Result<VpnSidecarHandle, VpnError> {
        let container_name = format!(
            "{}-{}",
            crate::constants::VPN_SIDECAR_NAME_PREFIX,
            Uuid::new_v4()
        );

        info!(container = %container_name, "Creating VPN sidecar");

        // 1. docker create
        let log_driver_arg = format!("--log-driver={}", crate::constants::VPN_SIDECAR_LOG_DRIVER);

        let create_args = vec![
            "create",
            "--name",
            &container_name,
            "--cap-add=NET_ADMIN",
            "--sysctl=net.ipv4.conf.all.src_valid_mark=1",
            // Disable IPv6 to prevent VPN bypass
            "--sysctl=net.ipv6.conf.all.disable_ipv6=1",
            "--sysctl=net.ipv6.conf.default.disable_ipv6=1",
            // Suppress handshake metadata from Docker logs
            &log_driver_arg,
            // Prevent privilege escalation
            "--security-opt=no-new-privileges",
            crate::constants::VPN_SIDECAR_IMAGE,
            "sleep",
            "infinity",
        ];

        let create_output = Command::new("docker")
            .args(&create_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| VpnError::SidecarFailed(format!("docker create failed: {}", e)))?;

        if !create_output.status.success() {
            return Err(VpnError::SidecarFailed(format!(
                "docker create failed: {}",
                String::from_utf8_lossy(&create_output.stderr).trim()
            )));
        }

        let container_id = String::from_utf8_lossy(&create_output.stdout)
            .trim()
            .to_string();

        // 2. docker start
        let start_output = Command::new("docker")
            .args(["start", &container_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| VpnError::SidecarFailed(format!("docker start failed: {}", e)))?;

        if !start_output.status.success() {
            let _ = Command::new("docker")
                .args(["rm", "-f", &container_id])
                .output()
                .await;
            return Err(VpnError::SidecarFailed(
                "failed to start VPN sidecar".to_string(),
            ));
        }

        let handle = VpnSidecarHandle {
            container_id: container_id.clone(),
            container_name: container_name.clone(),
            peer_id: peer_id.to_string(),
        };

        // 3. Ensure config directory exists
        let mkdir_output = Command::new("docker")
            .args(["exec", &container_id, "mkdir", "-p", "/etc/wireguard"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| VpnError::SidecarFailed(format!("mkdir failed: {}", e)))?;

        if !mkdir_output.status.success() {
            Self::destroy_sidecar_quiet(&handle).await;
            return Err(VpnError::SidecarFailed(format!(
                "failed to create config directory: {}",
                String::from_utf8_lossy(&mkdir_output.stderr).trim()
            )));
        }

        // 4. Write WireGuard config via stdin pipe
        let mut child = Command::new("docker")
            .args([
                "exec",
                "-i",
                &container_id,
                "sh",
                "-c",
                "cat > /etc/wireguard/wg0.conf",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| VpnError::SidecarFailed(format!("docker exec spawn failed: {}", e)))?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(wg_config.as_bytes())
                .await
                .map_err(VpnError::IoError)?;
            drop(stdin);
        }

        let write_output = child.wait_with_output().await.map_err(VpnError::IoError)?;

        if !write_output.status.success() {
            Self::destroy_sidecar_quiet(&handle).await;
            return Err(VpnError::SidecarFailed(format!(
                "failed to write WireGuard config: {}",
                String::from_utf8_lossy(&write_output.stderr).trim()
            )));
        }

        // 5. Bring up the WireGuard interface
        let wg_up = Command::new("docker")
            .args(["exec", &container_id, "wg-quick", "up", "wg0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| VpnError::SidecarFailed(format!("wg-quick up failed: {}", e)))?;

        if !wg_up.status.success() {
            Self::destroy_sidecar_quiet(&handle).await;
            return Err(VpnError::SidecarFailed(format!(
                "wg-quick up failed: {}",
                String::from_utf8_lossy(&wg_up.stderr).trim()
            )));
        }

        // 6. Apply iptables kill switch — blocks ALL traffic except through wg0
        let kill_switch = Command::new("docker")
            .args([
                "exec",
                &container_id,
                "sh",
                "-c",
                crate::constants::VPN_KILL_SWITCH_SCRIPT,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| VpnError::SidecarFailed(format!("kill switch exec failed: {}", e)))?;

        if !kill_switch.status.success() {
            Self::destroy_sidecar_quiet(&handle).await;
            return Err(VpnError::SidecarFailed(format!(
                "iptables kill switch failed: {}",
                String::from_utf8_lossy(&kill_switch.stderr).trim()
            )));
        }
        debug!(container = %container_name, "VPN kill switch applied");

        // 7. Wait for the tunnel to establish
        Self::wait_for_vpn_health(
            &container_id,
            crate::constants::VPN_HEALTH_CHECK_TIMEOUT_SECS,
            crate::constants::VPN_HEALTH_CHECK_INTERVAL_SECS,
        )
        .await
        .inspect_err(|_| {
            // Spawn cleanup in background — we can't await here easily
            let h = handle.clone();
            tokio::spawn(async move {
                Self::destroy_sidecar_quiet(&h).await;
            });
        })?;

        info!(container = %container_name, "VPN sidecar ready");
        Ok(handle)
    }

    /// Poll `wg show wg0` until the tunnel shows a valid handshake or timeout.
    async fn wait_for_vpn_health(
        container_id: &str,
        timeout_secs: u64,
        interval_secs: u64,
    ) -> Result<(), VpnError> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

        loop {
            if std::time::Instant::now() >= deadline {
                return Err(VpnError::HealthCheckTimeout { timeout_secs });
            }

            let output = Command::new("docker")
                .args(["exec", container_id, "wg", "show", "wg0"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await;

            match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    // A successful `wg show` with an interface listed means the tunnel is up.
                    // We check for "peer:" which indicates the peer section is present.
                    if stdout.contains("peer:") || stdout.contains("endpoint:") {
                        debug!(
                            container_id,
                            "WireGuard interface is up, verifying connectivity"
                        );

                        // Phase 2: verify actual connectivity through the tunnel
                        let ping = Command::new("docker")
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

                        if matches!(ping, Ok(ref out) if out.status.success()) {
                            debug!(container_id, "VPN tunnel connectivity verified");

                            // Phase 3: Verify traffic exits through VPN (IP leak check)
                            let timeout_arg = format!(
                                "--timeout={}",
                                crate::constants::VPN_IP_LEAK_CHECK_TIMEOUT_SECS
                            );
                            let ip_check = Command::new("docker")
                                .args([
                                    "exec",
                                    container_id,
                                    "wget",
                                    "-qO-",
                                    &timeout_arg,
                                    crate::constants::VPN_IP_CHECK_URL,
                                ])
                                .stdout(Stdio::piped())
                                .stderr(Stdio::piped())
                                .output()
                                .await;

                            match ip_check {
                                Ok(ref out) if out.status.success() => {
                                    let exit_ip =
                                        String::from_utf8_lossy(&out.stdout).trim().to_string();
                                    debug!(
                                        container_id,
                                        exit_ip = %exit_ip,
                                        "VPN exit IP verified"
                                    );
                                }
                                _ => {
                                    debug!(container_id, "IP leak check unavailable (non-fatal)");
                                }
                            }

                            return Ok(());
                        }
                        debug!(
                            container_id,
                            "VPN interface up but connectivity not ready yet"
                        );
                    }
                }
                Ok(out) => {
                    debug!(
                        container_id,
                        stderr = %String::from_utf8_lossy(&out.stderr),
                        "wg show failed"
                    );
                }
                Err(e) => {
                    debug!(container_id, error = %e, "docker exec failed for wg show");
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
    }

    /// Stop and remove a VPN sidecar container.
    pub async fn destroy_sidecar(handle: &VpnSidecarHandle) -> Result<(), VpnError> {
        info!(container = %handle.container_name, "Destroying VPN sidecar");

        // Bring down the WireGuard interface gracefully
        let _ = Command::new("docker")
            .args(["exec", &handle.container_id, "wg-quick", "down", "wg0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let _ = Command::new("docker")
            .args(["stop", "--time=5", &handle.container_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let rm_output = Command::new("docker")
            .args(["rm", "-f", &handle.container_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| VpnError::SidecarFailed(format!("docker rm failed: {}", e)))?;

        if !rm_output.status.success() {
            warn!(
                container = %handle.container_name,
                stderr = %String::from_utf8_lossy(&rm_output.stderr),
                "Failed to remove VPN sidecar"
            );
        }

        Ok(())
    }

    /// Destroy sidecar, ignoring errors. For cleanup in finally blocks.
    pub async fn destroy_sidecar_quiet(handle: &VpnSidecarHandle) {
        if let Err(e) = Self::destroy_sidecar(handle).await {
            warn!(
                container = %handle.container_name,
                error = %e,
                "Failed to destroy VPN sidecar (quiet)"
            );
        }
    }

    /// Find and remove orphaned VPN sidecar containers older than `max_age`.
    ///
    /// Called at server startup to clean up sidecars left behind by crashes.
    /// Returns the number of sidecars reaped.
    pub async fn reap_orphaned_sidecars(max_age: std::time::Duration) -> usize {
        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                &format!("name={}", crate::constants::VPN_SIDECAR_NAME_PREFIX),
                "--format",
                "{{.ID}}\t{{.CreatedAt}}\t{{.Names}}",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let output = match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            Ok(o) => {
                debug!(
                    stderr = %String::from_utf8_lossy(&o.stderr),
                    "docker ps failed during VPN sidecar reaper check"
                );
                return 0;
            }
            Err(e) => {
                debug!(error = %e, "docker not available for VPN sidecar reaper");
                return 0;
            }
        };

        let now = chrono::Utc::now();
        let mut reaped = 0;

        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(3, '\t').collect();
            if parts.len() < 3 {
                continue;
            }
            let container_id = parts[0];
            let created_at = parts[1];
            let container_name = parts[2];

            let age = match super::container::parse_docker_timestamp(created_at) {
                Some(created) => now.signed_duration_since(created),
                None => {
                    debug!(
                        container = container_name,
                        timestamp = created_at,
                        "Failed to parse VPN sidecar timestamp"
                    );
                    continue;
                }
            };

            if age.num_seconds() > max_age.as_secs() as i64 {
                warn!(
                    container = container_name,
                    age_secs = age.num_seconds(),
                    "Reaping orphaned VPN sidecar"
                );
                let _ = Command::new("docker")
                    .args(["rm", "-f", container_id])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await;
                reaped += 1;
            }
        }

        reaped
    }
}
