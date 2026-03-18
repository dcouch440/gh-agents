//! Container and VPN sidecar lifecycle management for DAG step execution.
//!
//! Handles creating, running, and destroying Docker containers with optional
//! WireGuard VPN sidecars for isolated network environments.

mod tests;

use anyhow::anyhow;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::execution::{
    ContainerConfig, ContainerHandle, ContainerManager, VpnSidecarHandle, VpnSidecarManager,
    WgEasyClient,
};
use crate::server::hub::error::HubError;

use super::utils::ContainerExecutionConfig;

/// A managed container that may include a VPN sidecar.
pub(crate) struct ManagedContainer {
    /// The agent container handle (for tool execution).
    pub agent_handle: ContainerHandle,
    /// Optional VPN sidecar (must be cleaned up separately).
    vpn_sidecar: Option<VpnSidecarHandle>,
}

/// Create a container if config is present, with optional VPN sidecar.
///
/// When `workspace_manager` is provided and the config has `workflow_id` + `run_id`,
/// the container gets a JuiceFS workspace bind-mounted at `/workspace/` instead of
/// a git clone.
///
/// Returns `Ok(None)` if config is `None` (local execution).
/// Returns `Ok(Some(managed))` on success, `Err` on failure.
pub(crate) async fn create_optional_container(
    config: Option<&ContainerExecutionConfig>,
    wg_client: Option<&WgEasyClient>,
    label: &str,
    workspace_manager: Option<&crate::server::services::workspace::WorkspaceManager>,
) -> Result<Option<ManagedContainer>, HubError> {
    let Some(cc) = config else {
        return Ok(None);
    };

    // If VPN is enabled, create a sidecar first
    let vpn_sidecar = if cc.vpn_enabled {
        let wg = wg_client.ok_or_else(|| {
            HubError::Internal(anyhow!("VPN enabled but wg-easy client not configured"))
        })?;

        use crate::execution::vpn::retry::vpn_with_retry;

        let peer_name = format!("{}-{}", label, Uuid::new_v4());
        let peer = vpn_with_retry(|| wg.create_peer(&peer_name))
            .await
            .map_err(|e| HubError::Internal(anyhow!("VPN peer creation failed: {}", e)))?;

        let peer_id_for_config = peer.id.clone();
        let peer_config = vpn_with_retry(|| wg.get_peer_config(&peer_id_for_config))
            .await
            .map_err(|e| HubError::Internal(anyhow!("VPN peer config failed: {}", e)))?;

        crate::execution::vpn::validate_wg_config(&peer_config)
            .map_err(|e| HubError::Internal(anyhow!("VPN config validation failed: {}", e)))?;

        let sidecar = match VpnSidecarManager::create_sidecar(&peer_config, &peer.id).await {
            Ok(s) => s,
            Err(e) => {
                warn!(peer_id = %peer.id, error = %e, "VPN sidecar failed, cleaning up peer");
                if let Some(wg) = wg_client {
                    if let Err(del_err) = wg.delete_peer(&peer.id).await {
                        warn!(
                            peer_id = %peer.id,
                            error = %del_err,
                            "Failed to clean up orphaned VPN peer"
                        );
                    }
                }
                return Err(HubError::Internal(anyhow!(
                    "VPN sidecar creation failed: {}",
                    e
                )));
            }
        };

        info!(
            container = %sidecar.container_name,
            peer_id = %peer.id,
            label,
            "VPN sidecar ready"
        );
        Some(sidecar)
    } else {
        None
    };

    // Resolve workspace mount path if JuiceFS is available
    let workspace_mount =
        if let (Some(wf_id), Some(run_id), Some(mgr)) = (cc.workflow_id, cc.run_id, workspace_manager) {
            match mgr.create_run_workspace(wf_id, run_id) {
                Ok(path) => {
                    info!(
                        workflow_id = %wf_id,
                        run_id = %run_id,
                        path = %path.display(),
                        "Resolved workspace mount path"
                    );
                    Some(path.to_string_lossy().to_string())
                }
                Err(e) => {
                    warn!(error = %e, "Failed to create workspace directory, falling back to git clone");
                    None
                }
            }
        } else {
            None
        };

    // Build container config, optionally sharing VPN sidecar's network
    let container_config = ContainerConfig {
        clone_url: cc.clone_url.clone(),
        branch: cc.branch.clone(),
        github_token: cc.github_token.clone(),
        image: cc
            .image
            .clone()
            .unwrap_or_else(|| crate::constants::CONTAINER_DEFAULT_IMAGE.to_string()),
        memory_limit: cc
            .memory_limit
            .clone()
            .unwrap_or_else(|| crate::constants::CONTAINER_DEFAULT_MEMORY.to_string()),
        cpu_limit: cc
            .cpu_limit
            .clone()
            .unwrap_or_else(|| crate::constants::CONTAINER_DEFAULT_CPUS.to_string()),
        network_mode: vpn_sidecar
            .as_ref()
            .map(|s| format!("container:{}", s.container_id)),
        workspace_mount,
        ..ContainerConfig::default()
    };

    use crate::execution::container::retry::container_with_retry;

    let container_mgr = ContainerManager::real();
    match container_with_retry(|| container_mgr.create_container(&container_config)).await {
        Ok(handle) => {
            info!(container = %handle.container_name(), label, "Created container");
            Ok(Some(ManagedContainer {
                agent_handle: handle,
                vpn_sidecar,
            }))
        }
        Err(e) => {
            // Clean up VPN sidecar if agent container creation fails
            if let Some(ref sidecar) = vpn_sidecar {
                VpnSidecarManager::destroy_sidecar_quiet(sidecar).await;
            }
            error!(label, error = %e, "Failed to create container");
            Err(HubError::Internal(anyhow!(
                "Container creation failed: {}",
                e
            )))
        }
    }
}

/// Destroy a managed container (agent + optional VPN sidecar + peer cleanup).
pub(crate) async fn destroy_optional_container(
    managed: &Option<ManagedContainer>,
    wg_client: Option<&WgEasyClient>,
) {
    let Some(ref mc) = managed else { return };

    // 1. Destroy agent container first
    ContainerManager::destroy_container_quiet(&mc.agent_handle).await;

    // 2. Destroy VPN sidecar if present
    if let Some(ref sidecar) = mc.vpn_sidecar {
        VpnSidecarManager::destroy_sidecar_quiet(sidecar).await;

        // 3. Delete wg-easy peer (with retry)
        if let Some(wg) = wg_client {
            use crate::execution::vpn::retry::vpn_with_retry;
            let peer_id = sidecar.peer_id.clone();
            if let Err(e) = vpn_with_retry(|| wg.delete_peer(&peer_id)).await {
                warn!(peer_id = %sidecar.peer_id, error = %e, "Failed to delete VPN peer");
            }
        }
    }
}

/// Run an execution future with a VPN tunnel watchdog.
///
/// If the managed container has a VPN sidecar, races the execution against
/// a tunnel monitor. If the tunnel drops, execution is cancelled and
/// an error is returned. If no VPN sidecar is present, execution runs directly.
pub(crate) async fn run_with_vpn_watchdog<F, T>(
    managed: &Option<ManagedContainer>,
    execution: F,
) -> Result<T, HubError>
where
    F: std::future::Future<Output = Result<T, HubError>>,
{
    use crate::execution::vpn_sidecar::watchdog::monitor_vpn_tunnel;

    let sidecar_id = managed
        .as_ref()
        .and_then(|mc| mc.vpn_sidecar.as_ref())
        .map(|s| s.container_id.as_str());

    match sidecar_id {
        Some(id) => {
            tokio::select! {
                result = execution => result,
                vpn_err = monitor_vpn_tunnel(
                    id,
                    crate::constants::VPN_WATCHDOG_INTERVAL_SECS,
                    crate::constants::VPN_WATCHDOG_MAX_FAILURES,
                ) => {
                    Err(HubError::Internal(anyhow!(
                        "VPN tunnel dropped during execution: {}", vpn_err
                    )))
                }
            }
        }
        None => execution.await,
    }
}
