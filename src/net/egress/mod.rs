//! The single gate for agent-driven outbound HTTP.
//!
//! Web tools run in the server process, not in an agent container, so the
//! per-step WireGuard sidecar does not cover them. This module is what does:
//! it hands out an HTTP client only when egress is permitted, and refuses
//! otherwise.
//!
//! ## Fail-closed
//!
//! The default mode is [`EgressMode::Vpn`]. If the tunnel is not configured,
//! not healthy, or not proven to be carrying traffic, [`client`] returns an
//! error and the calling tool reports it. There is deliberately no fallback
//! to a direct connection: a silent downgrade would be indistinguishable from
//! success while leaking the agent's queries.
//!
//! [`EgressMode::Direct`] exists for local development and must be opted into
//! explicitly by setting `NEXOR_WEB_EGRESS_MODE=direct`. It is refused in
//! production.

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::Client;
use thiserror::Error;

#[cfg(test)]
mod tests;

/// How outbound web-tool traffic is allowed to leave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EgressMode {
    /// Route through the VPN egress proxy. Refuse if it is not healthy.
    #[default]
    Vpn,
    /// Connect directly. Development only, and never in production.
    Direct,
}

impl EgressMode {
    /// Parse the `NEXOR_WEB_EGRESS_MODE` value.
    ///
    /// Anything unrecognised is treated as `Vpn`: an unparseable setting must
    /// fail closed, never open.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("direct") => EgressMode::Direct,
            _ => EgressMode::Vpn,
        }
    }
}

/// Why a web request may not leave.
#[derive(Debug, Error)]
pub enum EgressError {
    #[error("web egress is unavailable: the VPN egress proxy is not configured")]
    NotConfigured,

    #[error("web egress is unavailable: the VPN tunnel is not healthy ({0})")]
    Unhealthy(String),

    #[error("direct web egress is not permitted in production")]
    DirectRefusedInProduction,

    #[error("failed to build the HTTP client: {0}")]
    ClientBuild(#[from] reqwest::Error),
}

/// Runtime egress configuration, resolved once at startup.
#[derive(Debug, Clone)]
pub struct EgressConfig {
    pub mode: EgressMode,
    /// Proxy URL, e.g. `http://127.0.0.1:3128`. Required in `Vpn` mode.
    pub proxy_url: Option<String>,
    /// Whether this process is running in production.
    pub is_production: bool,
}

static CONFIG: OnceLock<EgressConfig> = OnceLock::new();

/// Install the egress configuration. Called once during startup.
///
/// Later calls are ignored, so a test or a second initialization cannot widen
/// what egress is permitted.
pub fn install(config: EgressConfig) {
    let _ = CONFIG.set(config);
}

/// The installed configuration, or the fail-closed default when startup has
/// not run (as in unit tests that never call [`install`]).
fn config() -> EgressConfig {
    CONFIG.get().cloned().unwrap_or(EgressConfig {
        mode: EgressMode::Vpn,
        proxy_url: None,
        is_production: false,
    })
}

/// Build an HTTP client for agent-driven web requests.
///
/// # Errors
///
/// Returns [`EgressError`] when egress is not permitted. Callers must surface
/// that to the agent rather than retrying without the proxy.
pub fn client(timeout: Duration) -> Result<Client, EgressError> {
    client_from(&config(), timeout)
}

/// [`client`], against an explicit configuration. Separated so the policy is
/// testable without touching process-global state.
pub fn client_from(cfg: &EgressConfig, timeout: Duration) -> Result<Client, EgressError> {
    client_from_builder(cfg, base_builder(timeout))
}

/// Like [`client`], but never follows redirects itself.
///
/// The page fetcher follows them manually so it can re-validate every hop: a
/// public URL redirecting to a private address is the standard way to turn an
/// allowed fetch into an internal one.
pub fn client_no_redirect(timeout: Duration) -> Result<Client, EgressError> {
    let cfg = config();
    let built = client_from_builder(
        &cfg,
        base_builder(timeout).redirect(reqwest::redirect::Policy::none()),
    )?;
    Ok(built)
}

/// Apply the egress policy to an already-configured builder.
fn client_from_builder(
    cfg: &EgressConfig,
    builder: reqwest::ClientBuilder,
) -> Result<Client, EgressError> {
    match cfg.mode {
        EgressMode::Direct => {
            if cfg.is_production {
                return Err(EgressError::DirectRefusedInProduction);
            }
            Ok(builder.build()?)
        }
        EgressMode::Vpn => {
            let url = cfg.proxy_url.as_deref().ok_or(EgressError::NotConfigured)?;
            let proxy = reqwest::Proxy::all(url).map_err(EgressError::ClientBuild)?;
            Ok(builder.proxy(proxy).build()?)
        }
    }
}

/// Shared client settings for every outbound web request.
fn base_builder(timeout: Duration) -> reqwest::ClientBuilder {
    Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(
            crate::constants::WEB_CONNECT_TIMEOUT_SECS,
        ))
        .user_agent(crate::constants::WEB_USER_AGENT)
        // Ambient proxy variables must never influence where an agent's
        // traffic goes; the route is decided here and nowhere else.
        .no_proxy()
}
