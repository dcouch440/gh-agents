//! wg-easy REST API client for managing WireGuard VPN peers.
//!
//! Each agent container can be paired with a VPN sidecar. This module handles
//! peer lifecycle (create, get config, delete) via the wg-easy HTTP API.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

use super::container::RedactedString;

#[cfg(test)]
mod integration_tests;
pub mod retry;
mod tests;

// ── Errors ─────────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum VpnError {
    #[error("wg-easy API unreachable: {0}")]
    ApiUnreachable(String),

    #[error("wg-easy authentication failed")]
    AuthFailed,

    #[error("peer creation failed: {reason}")]
    PeerCreationFailed { reason: String },

    #[error("peer deletion failed for {peer_id}: {reason}")]
    PeerDeletionFailed { peer_id: String, reason: String },

    #[error("peer config retrieval failed for {peer_id}: {reason}")]
    ConfigRetrievalFailed { peer_id: String, reason: String },

    #[error("VPN sidecar container failed: {0}")]
    SidecarFailed(String),

    #[error("VPN health check failed after {timeout_secs}s")]
    HealthCheckTimeout { timeout_secs: u64 },

    #[error("http request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

// ── Config ─────────────────────────────────────────────────────────────────

/// Configuration for connecting to a wg-easy instance.
#[derive(Debug, Clone)]
pub struct WgEasyConfig {
    /// Base URL of the wg-easy API (e.g., "http://localhost:51821").
    pub base_url: String,
    /// Password for wg-easy session authentication.
    pub password: RedactedString,
    /// HTTP request timeout in seconds.
    pub timeout_secs: u64,
}

impl WgEasyConfig {
    /// Build config from environment variables.
    ///
    /// Returns `None` if `WGEASY_API_URL` is not set.
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("WGEASY_API_URL").ok()?;
        let password = std::env::var("WGEASY_PASSWORD").unwrap_or_default();
        Some(Self {
            base_url,
            password: RedactedString::new(password),
            timeout_secs: crate::constants::WGEASY_API_TIMEOUT_SECS,
        })
    }
}

// ── API Types ──────────────────────────────────────────────────────────────

/// Response from wg-easy when creating or listing a peer.
#[derive(Debug, Deserialize)]
pub struct WgEasyPeer {
    pub id: String,
    pub name: String,
    pub address: String,
    pub enabled: bool,
}

/// Request body for creating a peer.
#[derive(Serialize)]
struct CreatePeerRequest {
    name: String,
}

/// Request body for session authentication.
#[derive(Serialize)]
struct SessionRequest {
    password: String,
}

// ── Client ─────────────────────────────────────────────────────────────────

/// Client for the wg-easy REST API.
///
/// Uses session-based authentication (cookie). The client will authenticate
/// lazily on the first API call and re-authenticate if a session expires.
pub struct WgEasyClient {
    config: WgEasyConfig,
    http: reqwest::Client,
    authenticated: AtomicBool,
}

impl WgEasyClient {
    pub fn new(config: WgEasyConfig) -> Self {
        let http = reqwest::Client::builder()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("failed to build reqwest client");
        Self {
            config,
            http,
            authenticated: AtomicBool::new(false),
        }
    }

    /// Returns whether the client has an active session.
    pub fn is_authenticated(&self) -> bool {
        self.authenticated.load(Ordering::Acquire)
    }

    /// Authenticate with wg-easy and store the session cookie.
    async fn authenticate(&self) -> Result<(), VpnError> {
        let url = format!("{}/api/session", self.config.base_url);
        debug!(url = %url, "Authenticating with wg-easy");

        let resp = self
            .http
            .post(&url)
            .json(&SessionRequest {
                password: self.config.password.expose().to_string(),
            })
            .send()
            .await
            .map_err(|e| VpnError::ApiUnreachable(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(VpnError::AuthFailed);
        }

        self.authenticated.store(true, Ordering::Release);
        debug!("wg-easy authentication successful");
        Ok(())
    }

    /// Authenticate only if not already authenticated.
    async fn ensure_authenticated(&self) -> Result<(), VpnError> {
        if self.authenticated.load(Ordering::Acquire) {
            return Ok(());
        }
        self.authenticate().await
    }

    /// Clear the cached session so the next call re-authenticates.
    fn invalidate_session(&self) {
        self.authenticated.store(false, Ordering::Release);
    }

    /// Check if a response status indicates an expired/invalid session.
    fn is_auth_expired(status: reqwest::StatusCode) -> bool {
        status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN
    }

    /// Create a new WireGuard peer. Returns peer metadata.
    pub async fn create_peer(&self, name: &str) -> Result<WgEasyPeer, VpnError> {
        self.ensure_authenticated().await?;

        let url = format!("{}/api/wireguard/client", self.config.base_url);
        info!(name = %name, "Creating wg-easy peer");

        let resp = self
            .http
            .post(&url)
            .json(&CreatePeerRequest {
                name: name.to_string(),
            })
            .send()
            .await?;

        // Re-auth once on session expiry
        if Self::is_auth_expired(resp.status()) {
            debug!("Session expired during create_peer, re-authenticating");
            self.invalidate_session();
            self.ensure_authenticated().await?;
            let resp = self
                .http
                .post(&url)
                .json(&CreatePeerRequest {
                    name: name.to_string(),
                })
                .send()
                .await?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(VpnError::PeerCreationFailed {
                    reason: format!("HTTP {}: {}", status, body),
                });
            }
            let peer: WgEasyPeer = resp
                .json()
                .await
                .map_err(|e| VpnError::PeerCreationFailed {
                    reason: format!("invalid response: {}", e),
                })?;
            info!(peer_id = %peer.id, name = %peer.name, address = %peer.address, "Peer created");
            return Ok(peer);
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VpnError::PeerCreationFailed {
                reason: format!("HTTP {}: {}", status, body),
            });
        }

        let peer: WgEasyPeer = resp
            .json()
            .await
            .map_err(|e| VpnError::PeerCreationFailed {
                reason: format!("invalid response: {}", e),
            })?;

        info!(peer_id = %peer.id, name = %peer.name, address = %peer.address, "Peer created");
        Ok(peer)
    }

    /// Get the WireGuard config file content for a peer.
    pub async fn get_peer_config(&self, peer_id: &str) -> Result<String, VpnError> {
        self.ensure_authenticated().await?;

        let url = format!(
            "{}/api/wireguard/client/{}/configuration",
            self.config.base_url, peer_id
        );
        debug!(peer_id = %peer_id, "Fetching peer config");

        let resp = self.http.get(&url).send().await?;

        // Re-auth once on session expiry
        if Self::is_auth_expired(resp.status()) {
            debug!("Session expired during get_peer_config, re-authenticating");
            self.invalidate_session();
            self.ensure_authenticated().await?;
            let resp = self.http.get(&url).send().await?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(VpnError::ConfigRetrievalFailed {
                    peer_id: peer_id.to_string(),
                    reason: format!("HTTP {}: {}", status, body),
                });
            }
            let config = resp
                .text()
                .await
                .map_err(|e| VpnError::ConfigRetrievalFailed {
                    peer_id: peer_id.to_string(),
                    reason: e.to_string(),
                })?;
            debug!(peer_id = %peer_id, config_len = config.len(), "Peer config retrieved");
            return Ok(config);
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VpnError::ConfigRetrievalFailed {
                peer_id: peer_id.to_string(),
                reason: format!("HTTP {}: {}", status, body),
            });
        }

        let config = resp
            .text()
            .await
            .map_err(|e| VpnError::ConfigRetrievalFailed {
                peer_id: peer_id.to_string(),
                reason: e.to_string(),
            })?;

        debug!(peer_id = %peer_id, config_len = config.len(), "Peer config retrieved");
        Ok(config)
    }

    /// List all WireGuard peers.
    pub async fn list_peers(&self) -> Result<Vec<WgEasyPeer>, VpnError> {
        self.ensure_authenticated().await?;

        let url = format!("{}/api/wireguard/client", self.config.base_url);
        debug!("Listing wg-easy peers");

        let resp = self.http.get(&url).send().await?;

        if Self::is_auth_expired(resp.status()) {
            self.invalidate_session();
            self.ensure_authenticated().await?;
            let resp = self.http.get(&url).send().await?;
            if !resp.status().is_success() {
                return Err(VpnError::ApiUnreachable(format!(
                    "list peers failed: HTTP {}",
                    resp.status()
                )));
            }
            return resp
                .json()
                .await
                .map_err(|e| VpnError::ApiUnreachable(format!("invalid response: {}", e)));
        }

        if !resp.status().is_success() {
            return Err(VpnError::ApiUnreachable(format!(
                "list peers failed: HTTP {}",
                resp.status()
            )));
        }

        resp.json()
            .await
            .map_err(|e| VpnError::ApiUnreachable(format!("invalid response: {}", e)))
    }

    /// Delete a peer by ID.
    pub async fn delete_peer(&self, peer_id: &str) -> Result<(), VpnError> {
        self.ensure_authenticated().await?;

        let url = format!("{}/api/wireguard/client/{}", self.config.base_url, peer_id);
        info!(peer_id = %peer_id, "Deleting wg-easy peer");

        let resp = self.http.delete(&url).send().await?;

        // Re-auth once on session expiry
        if Self::is_auth_expired(resp.status()) {
            debug!("Session expired during delete_peer, re-authenticating");
            self.invalidate_session();
            self.ensure_authenticated().await?;
            let resp = self.http.delete(&url).send().await?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(VpnError::PeerDeletionFailed {
                    peer_id: peer_id.to_string(),
                    reason: format!("HTTP {}: {}", status, body),
                });
            }
            info!(peer_id = %peer_id, "Peer deleted");
            return Ok(());
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VpnError::PeerDeletionFailed {
                peer_id: peer_id.to_string(),
                reason: format!("HTTP {}: {}", status, body),
            });
        }

        info!(peer_id = %peer_id, "Peer deleted");
        Ok(())
    }

    /// Reap orphaned wg-easy peers whose names match our prefix
    /// but have no matching running VPN sidecar container.
    pub async fn reap_orphaned_peers(&self) -> usize {
        let peers = match self.list_peers().await {
            Ok(p) => p,
            Err(e) => {
                debug!(error = %e, "Cannot list peers for reaper");
                return 0;
            }
        };

        let running_names = get_running_vpn_sidecar_names().await;
        let mut reaped = 0;

        for peer in peers {
            // Only reap peers with our naming convention
            let is_ours = peer
                .name
                .starts_with(crate::constants::VPN_SIDECAR_NAME_PREFIX)
                || peer
                    .name
                    .starts_with(crate::constants::CONTAINER_NAME_PREFIX);
            if !is_ours {
                continue;
            }

            // If no running sidecar references this peer name, it's orphaned
            let has_running_sidecar = running_names.iter().any(|n| peer.name.contains(n));
            if !has_running_sidecar {
                warn!(peer_id = %peer.id, peer_name = %peer.name, "Reaping orphaned VPN peer");
                if let Err(e) = self.delete_peer(&peer.id).await {
                    warn!(peer_id = %peer.id, error = %e, "Failed to reap VPN peer");
                } else {
                    reaped += 1;
                }
            }
        }

        reaped
    }

    /// Check if the wg-easy API is reachable.
    pub async fn health_check(&self) -> Result<(), VpnError> {
        self.authenticate().await
    }
}

/// Get names of currently running VPN sidecar containers.
async fn get_running_vpn_sidecar_names() -> Vec<String> {
    use std::process::Stdio;
    use tokio::process::Command;

    let output = Command::new("docker")
        .args([
            "ps",
            "--filter",
            &format!("name={}", crate::constants::VPN_SIDECAR_NAME_PREFIX),
            "--format",
            "{{.Names}}",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}
