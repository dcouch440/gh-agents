//! Persistent Docker container management for agent step execution.
//!
//! Each DAG step can optionally execute inside its own Docker container with a
//! cloned GitHub repo. The container stays alive across all tool calls within
//! the step, then is destroyed when the step completes.

use std::fmt;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;
use tracing::{debug, info, warn};
use uuid::Uuid;

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

// ── Exec Result ────────────────────────────────────────────────────────────

/// Result of executing a command inside a container.
#[derive(Debug, Clone)]
pub struct ContainerExecResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
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
                let result = ContainerExecResult {
                    success: output.status.success(),
                    exit_code: output.status.code().unwrap_or(-1),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    duration_ms,
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

/// Creates and destroys persistent Docker containers for agent steps.
pub struct ContainerManager;

impl ContainerManager {
    /// Create and start a persistent container, then clone the repo into it.
    pub async fn create_container(
        config: &ContainerConfig,
    ) -> Result<ContainerHandle, ContainerError> {
        let container_name = format!(
            "{}-{}",
            crate::constants::CONTAINER_NAME_PREFIX,
            Uuid::new_v4()
        );

        info!(container = %container_name, image = %config.image, "Creating persistent container");

        // 1. docker create
        let mut create_args = vec![
            "create".to_string(),
            "--name".to_string(),
            container_name.clone(),
            format!("--memory={}", config.memory_limit),
            format!("--cpus={}", config.cpu_limit),
            "-w".to_string(),
            config.workdir.clone(),
            format!("--env=GITHUB_TOKEN={}", config.github_token.expose()),
        ];
        for (k, v) in &config.env_vars {
            create_args.push(format!("--env={}={}", k, v));
        }
        if let Some(ref network) = config.network_mode {
            create_args.push(format!("--network={}", network));
        }
        create_args.push(config.image.clone());
        create_args.push("sleep".to_string());
        create_args.push("infinity".to_string());

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

        info!(
            container = %container_name,
            repo = %config.clone_url,
            "Container ready with cloned repo"
        );

        Ok(handle)
    }

    /// Stop and remove a container.
    pub async fn destroy_container(handle: &ContainerHandle) -> Result<(), ContainerError> {
        info!(container = %handle.container_name, "Destroying container");

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
}
