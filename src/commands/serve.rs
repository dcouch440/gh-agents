//! Server mode command handler

use anyhow::Result;
use std::net::SocketAddr;
use std::path::Path;
use tracing::{debug, info};

use crate::cli::Args;
use crate::config::load_config;
use crate::db::init_db;
use crate::logging::{init_logging_with_file, LOG_DIR};
use crate::server::start_server;

/// Run in server mode (HTTP + WebSocket)
pub async fn run_serve(args: Args) -> Result<()> {
    // Initialize logging with file output
    let log_path = args
        .config()
        .as_ref()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new(LOG_DIR).to_path_buf());
    let _guard = init_logging_with_file(Some(&log_path))?;

    info!("nexor server starting...");
    debug!("Debug logging enabled (verbosity: {})", args.verbose);

    // Load configuration
    let config = load_config().unwrap_or_default();

    // Initialize database
    let pool = init_db().await?;

    // Reap orphaned containers from previous crashes
    let reaped = crate::execution::ContainerManager::real()
        .reap_orphaned_containers(std::time::Duration::from_secs(
            crate::constants::CONTAINER_REAPER_MAX_AGE_SECS,
        ))
        .await;
    if reaped > 0 {
        info!("Reaped {} orphaned container(s)", reaped);
    }

    // Reap orphaned VPN sidecar containers
    let vpn_reaped = crate::execution::VpnSidecarManager::reap_orphaned_sidecars(
        std::time::Duration::from_secs(crate::constants::VPN_REAPER_MAX_AGE_SECS),
    )
    .await;
    if vpn_reaped > 0 {
        info!("Reaped {} orphaned VPN sidecar(s)", vpn_reaped);
    }

    // Reap orphaned wg-easy peers (only if wg-easy is configured)
    if let Some(wg_config) = crate::execution::WgEasyConfig::from_env() {
        let wg_client = crate::execution::WgEasyClient::new(wg_config);
        let peers_reaped = wg_client.reap_orphaned_peers().await;
        if peers_reaped > 0 {
            info!("Reaped {} orphaned VPN peer(s)", peers_reaped);
        }
    }

    // Server address from CLI
    let addr: SocketAddr = format!("0.0.0.0:{}", args.port()).parse()?;

    // Run server
    start_server(pool, config, addr).await?;

    info!("nexor shutting down");
    Ok(())
}
