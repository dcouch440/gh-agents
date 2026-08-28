//! Server mode command handler

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::cli::Args;
use crate::config::load_config;
use crate::db::init_db;
use crate::env::Env;
use crate::execution::vpn::WgEasyConfig;
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

    // Load environment once — everything reads from this struct
    let env = Arc::new(Env::load());

    // Install the web-egress policy before anything can make a request.
    // Until this runs the gate refuses everything, so an early failure is a
    // refused fetch rather than an unprotected one.
    let installed = crate::net::egress::install(crate::net::egress::EgressConfig {
        mode: env.web_egress_mode,
        proxy_url: env.vpn_proxy_url.clone(),
        is_production: env.is_production(),
    });
    if installed {
        info!(
            mode = ?env.web_egress_mode,
            proxy_configured = env.vpn_proxy_url.is_some(),
            "web egress policy installed"
        );
    } else {
        // First writer wins. Logging the intended policy as though it were
        // live would leave the operator's log asserting a rule that is not in
        // effect — which, for an egress gate, is worse than saying nothing.
        warn!(
            mode = ?env.web_egress_mode,
            "web egress policy was already installed; this configuration was NOT applied"
        );
    }

    // Load configuration
    let config = load_config().unwrap_or_default();

    // Initialize database
    let pool = init_db(&env).await?;

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
    if let Some(wg_config) = WgEasyConfig::from_env_config(&env) {
        let wg_client = crate::execution::WgEasyClient::new(wg_config);
        let peers_reaped = wg_client.reap_orphaned_peers().await;
        if peers_reaped > 0 {
            info!("Reaped {} orphaned VPN peer(s)", peers_reaped);
        }
    }

    // Server address from CLI
    let addr: SocketAddr = format!("0.0.0.0:{}", args.port()).parse()?;

    // Run server
    start_server(pool, config, addr, env).await?;

    info!("nexor shutting down");
    Ok(())
}
