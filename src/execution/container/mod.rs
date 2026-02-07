//! Persistent Docker container management for agent step execution.
//!
//! Each DAG step can optionally execute inside its own Docker container with a
//! cloned GitHub repo. The container stays alive across all tool calls within
//! the step, then is destroyed when the step completes.

use std::fmt;
use std::process::Stdio;
use std::sync::Arc;

use once_cell::sync::Lazy;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub mod retry;
mod tests;

// ── RedactedString ────────────────────────────────────────────────────────

/// A string that hides its value in Debug/Display output.
///
/// Use `.expose()` to access the inner value when you need it
/// (e.g., for constructing authenticated URLs or env vars).
#[derive(Clone)]
pub struct RedactedString(String);

impl RedactedString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Access the inner secret value. Only use where the actual value is needed.
    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for RedactedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl fmt::Display for RedactedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl Default for RedactedString {
    fn default() -> Self {
        Self(String::new())
    }
}

// ── Errors ─────────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("docker not available: {0}")]
    DockerNotAvailable(String),

    #[error("container creation failed: {0}")]
    CreationFailed(String),

    #[error("repo clone failed: {stderr}")]
    CloneFailed { stderr: String },

    #[error("command failed in container {container}: exit {exit_code}: {stderr}")]
    CommandFailed {
        container: String,
        exit_code: i32,
        stderr: String,
    },

    #[error("command timed out after {timeout_secs}s in container {container}")]
    Timeout {
        container: String,
        timeout_secs: u64,
    },

    #[error("container not running: {container}")]
    NotRunning { container: String },

    #[error("path not allowed: {path} — {reason}")]
    PathNotAllowed { path: String, reason: String },

    #[error("docker {operation} failed: {source}")]
    DockerSpawnFailed {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("container creation timed out after {timeout_secs}s for {container}")]
    CreateTimeout {
        container: String,
        timeout_secs: u64,
    },

    #[error("network disconnect failed for container {container}: {stderr}")]
    NetworkDisconnectFailed { container: String, stderr: String },

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

// ── Config ─────────────────────────────────────────────────────────────────

/// Configuration for creating a persistent agent container.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// Docker image (e.g., "nexor-agent:latest").
    pub image: String,
    /// GitHub repo clone URL (e.g., "https://github.com/owner/repo.git").
    pub clone_url: String,
    /// Branch to checkout after clone. None = default branch.
    pub branch: Option<String>,
    /// GitHub token for authenticated clone/push.
    pub github_token: RedactedString,
    /// Memory limit (e.g., "2g").
    pub memory_limit: String,
    /// CPU limit (e.g., "2.0").
    pub cpu_limit: String,
    /// Timeout for individual commands in seconds.
    pub command_timeout_secs: u64,
    /// Additional environment variables.
    pub env_vars: Vec<(String, String)>,
    /// Working directory inside container.
    pub workdir: String,
    /// Optional Docker network mode (e.g., "container:<vpn_sidecar_id>").
    pub network_mode: Option<String>,
    /// Whether to disconnect the container from all networks after the initial
    /// git clone. Defaults to `true`. Ignored when `network_mode` is `Some`.
    pub network_isolated: bool,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            image: crate::constants::CONTAINER_DEFAULT_IMAGE.to_string(),
            clone_url: String::new(),
            branch: None,
            github_token: RedactedString::default(),
            memory_limit: crate::constants::CONTAINER_DEFAULT_MEMORY.to_string(),
            cpu_limit: crate::constants::CONTAINER_DEFAULT_CPUS.to_string(),
            command_timeout_secs: crate::constants::CONTAINER_COMMAND_TIMEOUT_SECS,
            env_vars: Vec::new(),
            workdir: "/workspace".to_string(),
            network_mode: None,
            network_isolated: true,
        }
    }
}

// ── Security Utilities ────────────────────────────────────────────────────

/// Strip a token from git stderr output to prevent credential leaks in logs/errors.
pub fn sanitize_git_output(output: &str, token: &RedactedString) -> String {
    let secret = token.expose();
    if secret.is_empty() {
        return output.to_string();
    }
    output.replace(secret, "[REDACTED]")
}

/// Escape a path for safe use inside single-quoted shell arguments.
///
/// Handles the `'` character by ending the quote, inserting an escaped quote,
/// and reopening the quote: `can't` → `'can'\''t'`
pub fn shell_escape_path(path: &str) -> String {
    if !path.contains('\'') {
        return format!("'{}'", path);
    }
    let escaped = path.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

/// Validate that a container path is safe (no absolute paths, no `..` traversal).
pub fn validate_container_path(path: &str) -> Result<(), ContainerError> {
    if path.starts_with('/') {
        return Err(ContainerError::PathNotAllowed {
            path: path.to_string(),
            reason: "absolute paths are not allowed".to_string(),
        });
    }
    for component in path.split('/') {
        if component == ".." {
            return Err(ContainerError::PathNotAllowed {
                path: path.to_string(),
                reason: "path traversal (..) is not allowed".to_string(),
            });
        }
    }
    Ok(())
}

/// Disconnect a container from the Docker bridge network.
///
/// Removes all network access, preventing data exfiltration after the
/// initial git clone. Idempotent: returns Ok if the container is already
/// disconnected or the network doesn't exist.
async fn disconnect_bridge_network(
    container_id: &str,
    container_name: &str,
) -> Result<(), ContainerError> {
    let output = Command::new("docker")
        .args(["network", "disconnect", "-f", "bridge", container_id])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| ContainerError::DockerSpawnFailed {
            operation: "network disconnect",
            source: e,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        // Docker returns an error if the container is not connected — that's fine.
        if stderr.contains("is not connected to") {
            debug!(
                container = container_name,
                "Container already disconnected from bridge"
            );
            return Ok(());
        }
        return Err(ContainerError::NetworkDisconnectFailed {
            container: container_name.to_string(),
            stderr,
        });
    }

    info!(
        container = container_name,
        "Disconnected container from bridge network (network isolated)"
    );
    Ok(())
}

// ── Exec Result ────────────────────────────────────────────────────────────

/// Result of executing a command inside a container.
#[derive(Debug, Clone)]
pub struct ContainerExecResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    /// Whether stdout or stderr was truncated due to exceeding the output size limit.
    pub truncated: bool,
}

/// Truncate raw command output to `max_bytes`, returning the string and whether truncation occurred.
fn truncate_output(raw: &[u8], max_bytes: usize) -> (String, bool) {
    if raw.len() <= max_bytes {
        return (String::from_utf8_lossy(raw).to_string(), false);
    }
    let truncated = String::from_utf8_lossy(&raw[..max_bytes]).to_string();
    (
        format!("{}\n... [truncated, {} bytes total]", truncated, raw.len()),
        true,
    )
}

// ── Handle ─────────────────────────────────────────────────────────────────

/// Handle to a running persistent container. All tool calls go through this.
#[derive(Debug, Clone)]
pub struct ContainerHandle {
    container_id: String,
    container_name: String,
    workdir: String,
    command_timeout_secs: u64,
}

impl ContainerHandle {
    /// Get the container name (for logging).
    pub fn container_name(&self) -> &str {
        &self.container_name
    }

    /// Execute a command inside the container via `docker exec`.
    pub async fn exec(&self, command: &[&str]) -> Result<ContainerExecResult, ContainerError> {
        let start = std::time::Instant::now();

        let mut docker_args = vec!["exec", "-w", &self.workdir, &self.container_id];
        docker_args.extend(command);

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.command_timeout_secs),
            Command::new("docker")
                .args(&docker_args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(Ok(output)) => {
                let max = crate::constants::CONTAINER_MAX_OUTPUT_BYTES;
                let (stdout, stdout_truncated) = truncate_output(&output.stdout, max);
                let (stderr, stderr_truncated) = truncate_output(&output.stderr, max);
                let truncated = stdout_truncated || stderr_truncated;

                if truncated {
                    warn!(
                        container = %self.container_name,
                        stdout_bytes = output.stdout.len(),
                        stderr_bytes = output.stderr.len(),
                        max_bytes = max,
                        "Container output truncated"
                    );
                }

                let result = ContainerExecResult {
                    success: output.status.success(),
                    exit_code: output.status.code().unwrap_or(-1),
                    stdout,
                    stderr,
                    duration_ms,
                    truncated,
                };
                debug!(
                    container = %self.container_name,
                    exit_code = result.exit_code,
                    duration_ms,
                    "Container exec completed"
                );
                Ok(result)
            }
            Ok(Err(e)) => Err(ContainerError::DockerSpawnFailed {
                operation: "exec",
                source: e,
            }),
            Err(_) => {
                warn!(
                    container = %self.container_name,
                    timeout_secs = self.command_timeout_secs,
                    "Container command timed out"
                );
                Err(ContainerError::Timeout {
                    container: self.container_name.clone(),
                    timeout_secs: self.command_timeout_secs,
                })
            }
        }
    }

    /// Execute a shell command via `docker exec sh -c "..."`.
    pub async fn exec_shell(&self, shell_cmd: &str) -> Result<ContainerExecResult, ContainerError> {
        self.exec(&["sh", "-c", shell_cmd]).await
    }

    /// Read a file from the container.
    pub async fn read_file(&self, path: &str) -> Result<String, ContainerError> {
        let result = self.exec(&["cat", path]).await?;
        if !result.success {
            return Err(ContainerError::CommandFailed {
                container: self.container_name.clone(),
                exit_code: result.exit_code,
                stderr: result.stderr,
            });
        }
        Ok(result.stdout)
    }

    /// Write a file inside the container by piping content through stdin.
    pub async fn write_file(&self, path: &str, content: &str) -> Result<(), ContainerError> {
        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(path).parent() {
            if parent != std::path::Path::new("") && parent != std::path::Path::new("/") {
                let mkdir_cmd = format!(
                    "mkdir -p {}",
                    shell_escape_path(&parent.display().to_string())
                );
                self.exec_shell(&mkdir_cmd).await?;
            }
        }

        // Write via stdin pipe
        let mut child = Command::new("docker")
            .args([
                "exec",
                "-i",
                "-w",
                &self.workdir,
                &self.container_id,
                "sh",
                "-c",
                &format!("cat > {}", shell_escape_path(path)),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ContainerError::DockerSpawnFailed {
                operation: "exec (write_file)",
                source: e,
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(content.as_bytes())
                .await
                .map_err(ContainerError::IoError)?;
            drop(stdin); // Close stdin to signal EOF
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(ContainerError::IoError)?;

        if !output.status.success() {
            return Err(ContainerError::CommandFailed {
                container: self.container_name.clone(),
                exit_code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }

    /// List files in a directory inside the container.
    pub async fn list_files(&self, path: &str) -> Result<Vec<String>, ContainerError> {
        let result = self.exec(&["ls", "-1a", path]).await?;
        if !result.success {
            return Err(ContainerError::CommandFailed {
                container: self.container_name.clone(),
                exit_code: result.exit_code,
                stderr: result.stderr,
            });
        }
        Ok(result
            .stdout
            .lines()
            .filter(|l| !l.is_empty() && *l != "." && *l != "..")
            .map(|l| l.to_string())
            .collect())
    }

    /// Run a git command inside the container.
    pub async fn git(&self, args: &[&str]) -> Result<String, ContainerError> {
        let mut cmd = vec!["git"];
        cmd.extend(args);
        let result = self.exec(&cmd).await?;
        if !result.success {
            return Err(ContainerError::CommandFailed {
                container: self.container_name.clone(),
                exit_code: result.exit_code,
                stderr: result.stderr,
            });
        }
        Ok(result.stdout)
    }

    /// Check if the container is still running.
    pub async fn is_alive(&self) -> bool {
        let output = Command::new("docker")
            .args(["inspect", "--format={{.State.Running}}", &self.container_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;
        match output {
            Ok(o) => String::from_utf8_lossy(&o.stdout).trim() == "true",
            Err(_) => false,
        }
    }
}

// ── Manager ────────────────────────────────────────────────────────────────

/// Global semaphore limiting concurrent container creation operations.
///
/// Prevents Docker daemon resource exhaustion when for-each pipelines
/// attempt to create many containers simultaneously.
static CONTAINER_CREATE_SEMAPHORE: Lazy<Arc<Semaphore>> = Lazy::new(|| {
    Arc::new(Semaphore::new(
        crate::constants::CONTAINER_MAX_CONCURRENT_CREATES,
    ))
});

/// Build the `docker create` arguments for a container.
///
/// Separated for testability — security and resource flags are validated by unit tests.
fn build_create_args(container_name: &str, config: &ContainerConfig) -> Vec<String> {
    let mut args = vec![
        "create".to_string(),
        "--name".to_string(),
        container_name.to_string(),
        format!("--memory={}", config.memory_limit),
        format!("--cpus={}", config.cpu_limit),
        "--cap-drop=ALL".to_string(),
        "--security-opt=no-new-privileges".to_string(),
        "-w".to_string(),
        config.workdir.clone(),
        format!("--env=GITHUB_TOKEN={}", config.github_token.expose()),
    ];
    for (k, v) in &config.env_vars {
        args.push(format!("--env={}={}", k, v));
    }
    if let Some(ref network) = config.network_mode {
        args.push(format!("--network={}", network));
    }
    args.push(config.image.clone());
    args.push("sleep".to_string());
    args.push("infinity".to_string());
    args
}

/// Creates and destroys persistent Docker containers for agent steps.
pub struct ContainerManager;

impl ContainerManager {
    /// Create and start a persistent container, then clone the repo into it.
    ///
    /// Acquires a creation semaphore permit (queues if at capacity), then wraps
    /// the entire creation flow in a timeout. On timeout, attempts cleanup of
    /// the partially-created container.
    pub async fn create_container(
        config: &ContainerConfig,
    ) -> Result<ContainerHandle, ContainerError> {
        // Acquire creation semaphore (queue if at capacity)
        let _permit = CONTAINER_CREATE_SEMAPHORE
            .acquire()
            .await
            .expect("container creation semaphore closed");

        let timeout_secs = crate::constants::CONTAINER_CREATE_TIMEOUT_SECS;
        let timeout_duration = std::time::Duration::from_secs(timeout_secs);

        // Generate name before timeout so we can reference it in cleanup
        let container_name = format!(
            "{}-{}",
            crate::constants::CONTAINER_NAME_PREFIX,
            Uuid::new_v4()
        );
        let name_for_cleanup = container_name.clone();

        let result = tokio::time::timeout(
            timeout_duration,
            Self::create_container_inner(config, container_name),
        )
        .await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                warn!(
                    container = %name_for_cleanup,
                    timeout_secs,
                    "Container creation timed out, attempting cleanup"
                );
                let _ = Command::new("docker")
                    .args(["rm", "-f", &name_for_cleanup])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await;
                Err(ContainerError::CreateTimeout {
                    container: name_for_cleanup,
                    timeout_secs,
                })
            }
        }
    }

    /// Inner implementation of container creation (called within a timeout).
    async fn create_container_inner(
        config: &ContainerConfig,
        container_name: String,
    ) -> Result<ContainerHandle, ContainerError> {
        let create_start = std::time::Instant::now();

        info!(container = %container_name, image = %config.image, "Creating persistent container");

        // 1. docker create
        let create_args = build_create_args(&container_name, config);

        let create_output = Command::new("docker")
            .args(&create_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ContainerError::DockerNotAvailable(e.to_string()))?;

        if !create_output.status.success() {
            return Err(ContainerError::CreationFailed(
                String::from_utf8_lossy(&create_output.stderr)
                    .trim()
                    .to_string(),
            ));
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
            .map_err(|e| ContainerError::DockerSpawnFailed {
                operation: "start",
                source: e,
            })?;

        if !start_output.status.success() {
            let _ = Command::new("docker")
                .args(["rm", "-f", &container_id])
                .output()
                .await;
            return Err(ContainerError::CreationFailed(
                "Failed to start container".to_string(),
            ));
        }

        let handle = ContainerHandle {
            container_id: container_id.clone(),
            container_name: container_name.clone(),
            workdir: config.workdir.clone(),
            command_timeout_secs: config.command_timeout_secs,
        };

        // 3. Clone repository
        let clone_url = format!(
            "https://x-access-token:{}@{}",
            config.github_token.expose(),
            config.clone_url.trim_start_matches("https://")
        );
        let clone_result = handle
            .exec(&["git", "clone", "--depth=1", &clone_url, "."])
            .await;
        match clone_result {
            Ok(r) if !r.success => {
                Self::destroy_container_quiet(&handle).await;
                return Err(ContainerError::CloneFailed {
                    stderr: sanitize_git_output(&r.stderr, &config.github_token),
                });
            }
            Err(e) => {
                Self::destroy_container_quiet(&handle).await;
                return Err(e);
            }
            _ => {}
        }

        // 4. Checkout branch if specified
        if let Some(ref branch) = config.branch {
            let checkout_result = handle.exec(&["git", "checkout", branch]).await;
            if let Ok(r) = &checkout_result {
                if !r.success {
                    // Try fetching the remote branch
                    let _ = handle.exec(&["git", "fetch", "origin", branch]).await;
                    let _ = handle
                        .exec(&[
                            "git",
                            "checkout",
                            "-b",
                            branch,
                            &format!("origin/{}", branch),
                        ])
                        .await;
                }
            }
        }

        // 5. Configure git user
        let _ = handle
            .exec(&["git", "config", "user.email", "nexor@nexor.ai"])
            .await;
        let _ = handle
            .exec(&["git", "config", "user.name", "Nexor Agent"])
            .await;

        // 6. Network isolation: disconnect from bridge after clone completes
        if config.network_isolated && config.network_mode.is_none() {
            if let Err(e) = disconnect_bridge_network(&container_id, &container_name).await {
                warn!(
                    container = %container_name,
                    error = %e,
                    "Failed to isolate container network — continuing with network access"
                );
            }
        }

        let create_duration_ms = create_start.elapsed().as_millis() as u64;
        info!(
            container = %container_name,
            repo = %config.clone_url,
            duration_ms = create_duration_ms,
            "Container ready with cloned repo"
        );

        Ok(handle)
    }

    /// Stop and remove a container.
    pub async fn destroy_container(handle: &ContainerHandle) -> Result<(), ContainerError> {
        let destroy_start = std::time::Instant::now();

        let _ = Command::new("docker")
            .args(["stop", "--time=10", &handle.container_id])
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
            .map_err(|e| ContainerError::DockerSpawnFailed {
                operation: "rm",
                source: e,
            })?;

        if !rm_output.status.success() {
            warn!(
                container = %handle.container_name,
                stderr = %String::from_utf8_lossy(&rm_output.stderr),
                "Failed to remove container"
            );
        }

        let destroy_duration_ms = destroy_start.elapsed().as_millis() as u64;
        info!(
            container = %handle.container_name,
            duration_ms = destroy_duration_ms,
            "Container destroyed"
        );

        Ok(())
    }

    /// Destroy container, ignoring errors. For cleanup in finally blocks.
    pub async fn destroy_container_quiet(handle: &ContainerHandle) {
        if let Err(e) = Self::destroy_container(handle).await {
            warn!(
                container = %handle.container_name,
                error = %e,
                "Failed to destroy container (quiet)"
            );
        }
    }

    /// Find and kill orphaned `nexor-step-*` containers older than `max_age`.
    ///
    /// Called at server startup to clean up containers left behind by crashes.
    /// Returns the number of containers reaped.
    pub async fn reap_orphaned_containers(max_age: std::time::Duration) -> usize {
        let reap_start = std::time::Instant::now();

        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                &format!("name={}", crate::constants::CONTAINER_NAME_PREFIX),
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
                    "docker ps failed during reaper check"
                );
                return 0;
            }
            Err(e) => {
                debug!(error = %e, "docker not available for reaper");
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

            let age = match parse_docker_timestamp(created_at) {
                Some(created) => now.signed_duration_since(created),
                None => {
                    debug!(
                        container = container_name,
                        timestamp = created_at,
                        "Failed to parse container timestamp"
                    );
                    continue;
                }
            };

            if age.num_seconds() > max_age.as_secs() as i64 {
                warn!(
                    container = container_name,
                    age_secs = age.num_seconds(),
                    "Reaping orphaned container"
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

        let reap_duration_ms = reap_start.elapsed().as_millis() as u64;
        if reaped > 0 {
            info!(
                reaped,
                duration_ms = reap_duration_ms,
                "Reaper cycle complete"
            );
        }

        reaped
    }

    /// Spawn a background task that periodically reaps orphaned containers.
    ///
    /// The task runs every `interval` and calls `reap_orphaned_containers(max_age)`.
    /// It stops cleanly when `shutdown` is cancelled.
    pub fn spawn_reaper(
        max_age: std::time::Duration,
        interval: std::time::Duration,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // Skip the first immediate tick (startup reap already ran)
            ticker.tick().await;

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let reaped = Self::reap_orphaned_containers(max_age).await;
                        if reaped > 0 {
                            info!("Periodic reaper cleaned up {} orphaned container(s)", reaped);
                        }
                    }
                    _ = shutdown.cancelled() => {
                        info!("Container reaper shutting down");
                        break;
                    }
                }
            }
        })
    }
}

/// Parse a Docker `CreatedAt` timestamp (e.g., "2024-01-15 10:30:00 -0700 MST").
///
/// Docker uses a non-standard format with both numeric offset and timezone abbreviation.
/// We parse the numeric offset portion and ignore the abbreviation.
pub fn parse_docker_timestamp(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Docker format: "2024-01-15 10:30:00 -0700 MST"
    // We need: "2024-01-15 10:30:00 -0700" (drop the TZ abbreviation)
    let parts: Vec<&str> = s.splitn(4, ' ').collect();
    if parts.len() < 3 {
        return None;
    }
    let without_tz_name = format!("{} {} {}", parts[0], parts[1], parts[2]);
    chrono::DateTime::parse_from_str(&without_tz_name, "%Y-%m-%d %H:%M:%S %z")
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}
