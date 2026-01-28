//! Docker sandbox for isolated command execution

use crate::execution::ExecutionContext;
use std::path::PathBuf;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("docker not available: {0}")]
    DockerNotAvailable(String),

    #[error("sandbox image not found: {image}")]
    ImageNotFound { image: String },

    #[error("command failed with exit code {exit_code}: {stderr}")]
    CommandFailed { exit_code: i32, stderr: String },

    #[error("command timed out after {timeout_secs} seconds")]
    Timeout { timeout_secs: u64 },

    #[error("execution error: {0}")]
    ExecutionError(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Docker image to use
    pub image: String,
    /// Memory limit (e.g., "512m")
    pub memory_limit: String,
    /// CPU limit (e.g., "1.0" for 1 CPU)
    pub cpu_limit: String,
    /// Execution timeout in seconds
    pub timeout_secs: u64,
    /// Network access enabled
    pub network_enabled: bool,
    /// Additional environment variables
    pub env_vars: Vec<(String, String)>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image: "alpine:latest".to_string(),
            memory_limit: "512m".to_string(),
            cpu_limit: "1.0".to_string(),
            timeout_secs: 300, // 5 minutes
            network_enabled: false,
            env_vars: vec![],
        }
    }
}

impl SandboxConfig {
    /// Create config for read-only sandbox (safer for untrusted commands)
    pub fn read_only() -> Self {
        Self {
            network_enabled: false,
            ..Default::default()
        }
    }

    /// Create config with network access (for package installation, etc.)
    pub fn with_network() -> Self {
        Self {
            network_enabled: true,
            ..Default::default()
        }
    }

    /// Configure for resource-intensive tasks
    pub fn high_resources() -> Self {
        Self {
            memory_limit: "2g".to_string(),
            cpu_limit: "2.0".to_string(),
            timeout_secs: 600, // 10 minutes
            ..Default::default()
        }
    }

    /// Configure for quick, lightweight tasks
    pub fn minimal_resources() -> Self {
        Self {
            memory_limit: "128m".to_string(),
            cpu_limit: "0.5".to_string(),
            timeout_secs: 60, // 1 minute
            ..Default::default()
        }
    }

    /// Builder pattern for custom configuration
    pub fn builder() -> SandboxConfigBuilder {
        SandboxConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct SandboxConfigBuilder {
    config: SandboxConfig,
}

impl SandboxConfigBuilder {
    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.config.image = image.into();
        self
    }

    pub fn memory(mut self, limit: impl Into<String>) -> Self {
        self.config.memory_limit = limit.into();
        self
    }

    pub fn cpus(mut self, limit: impl Into<String>) -> Self {
        self.config.cpu_limit = limit.into();
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.config.timeout_secs = secs;
        self
    }

    pub fn network(mut self, enabled: bool) -> Self {
        self.config.network_enabled = enabled;
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.env_vars.push((key.into(), value.into()));
        self
    }

    pub fn build(self) -> SandboxConfig {
        self.config
    }
}

#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub container_id: String,
}

#[derive(Debug, Clone)]
pub struct MountSpec {
    pub host_path: PathBuf,
    pub container_path: String,
    pub read_only: bool,
}

impl MountSpec {
    pub fn read_write(host: impl Into<PathBuf>, container: impl Into<String>) -> Self {
        Self {
            host_path: host.into(),
            container_path: container.into(),
            read_only: false,
        }
    }

    pub fn read_only(host: impl Into<PathBuf>, container: impl Into<String>) -> Self {
        Self {
            host_path: host.into(),
            container_path: container.into(),
            read_only: true,
        }
    }
}

pub struct Sandbox {
    ctx: ExecutionContext,
    config: SandboxConfig,
}

impl Sandbox {
    pub fn new(ctx: ExecutionContext, config: SandboxConfig) -> Self {
        Self { ctx, config }
    }

    pub fn with_defaults(ctx: ExecutionContext) -> Self {
        Self::new(ctx, SandboxConfig::default())
    }

    /// Check if Docker is available
    pub async fn check_docker(&self) -> Result<(), SandboxError> {
        let output = Command::new("docker")
            .args(["version", "--format", "{{.Server.Version}}"])
            .output()
            .await
            .map_err(|e| SandboxError::DockerNotAvailable(e.to_string()))?;

        if !output.status.success() {
            return Err(SandboxError::DockerNotAvailable(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        tracing::debug!(
            version = %String::from_utf8_lossy(&output.stdout).trim(),
            "Docker available"
        );

        Ok(())
    }

    /// Execute a command in the sandbox
    pub async fn exec(&self, command: &[&str]) -> Result<SandboxResult, SandboxError> {
        self.check_docker().await?;

        let container_name = format!("nexor-sandbox-{}", Uuid::new_v4());
        let start = std::time::Instant::now();

        // Build docker run arguments
        let mut docker_args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "--name".to_string(),
            container_name.clone(),
            // Resource limits
            format!("--memory={}", self.config.memory_limit),
            format!("--cpus={}", self.config.cpu_limit),
            // Mount project directory
            format!("--volume={}:/workspace:rw", self.ctx.project_root.display()),
            "--workdir=/workspace".to_string(),
        ];

        // Network
        if !self.config.network_enabled {
            docker_args.push("--network=none".to_string());
        }

        // Environment variables
        for (key, value) in &self.config.env_vars {
            docker_args.push(format!("--env={}={}", key, value));
        }

        // Image
        docker_args.push(self.config.image.clone());

        // Command
        docker_args.extend(command.iter().map(|s| s.to_string()));

        tracing::debug!(
            command = ?command,
            container = %container_name,
            "Executing in sandbox"
        );

        // Execute with timeout
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
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
                let exit_code = output.status.code().unwrap_or(-1);
                let result = SandboxResult {
                    success: output.status.success(),
                    exit_code,
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    duration_ms,
                    container_id: container_name,
                };

                if result.success {
                    tracing::info!(
                        exit_code = result.exit_code,
                        duration_ms = result.duration_ms,
                        "Sandbox command succeeded"
                    );
                } else {
                    tracing::warn!(
                        exit_code = result.exit_code,
                        stderr = %result.stderr.chars().take(200).collect::<String>(),
                        "Sandbox command failed"
                    );
                }

                Ok(result)
            }
            Ok(Err(e)) => Err(SandboxError::ExecutionError(e)),
            Err(_) => {
                // Timeout - kill container
                let _ = Command::new("docker")
                    .args(["kill", &container_name])
                    .output()
                    .await;

                Err(SandboxError::Timeout {
                    timeout_secs: self.config.timeout_secs,
                })
            }
        }
    }

    /// Execute a shell command in the sandbox
    pub async fn exec_shell(&self, shell_command: &str) -> Result<SandboxResult, SandboxError> {
        self.exec(&["sh", "-c", shell_command]).await
    }

    /// Execute with explicit mount options
    pub async fn exec_with_mounts(
        &self,
        command: &[&str],
        mounts: &[MountSpec],
    ) -> Result<SandboxResult, SandboxError> {
        self.check_docker().await?;

        let container_name = format!("nexor-sandbox-{}", Uuid::new_v4());
        let start = std::time::Instant::now();

        let mut docker_args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "--name".to_string(),
            container_name.clone(),
            format!("--memory={}", self.config.memory_limit),
            format!("--cpus={}", self.config.cpu_limit),
        ];

        // Add mounts
        for mount in mounts {
            let mode = if mount.read_only { "ro" } else { "rw" };
            docker_args.push(format!(
                "--volume={}:{}:{}",
                mount.host_path.display(),
                mount.container_path,
                mode
            ));
        }

        // Default: always mount project dir if not in mounts
        let has_workspace = mounts.iter().any(|m| m.container_path == "/workspace");
        if !has_workspace {
            docker_args.push(format!(
                "--volume={}:/workspace:rw",
                self.ctx.project_root.display()
            ));
        }

        docker_args.push("--workdir=/workspace".to_string());

        if !self.config.network_enabled {
            docker_args.push("--network=none".to_string());
        }

        docker_args.push(self.config.image.clone());
        docker_args.extend(command.iter().map(|s| s.to_string()));

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            Command::new("docker").args(&docker_args).output(),
        )
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(Ok(output)) => Ok(SandboxResult {
                success: output.status.success(),
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                duration_ms,
                container_id: container_name,
            }),
            Ok(Err(e)) => Err(SandboxError::ExecutionError(e)),
            Err(_) => {
                let _ = Command::new("docker")
                    .args(["kill", &container_name])
                    .output()
                    .await;
                Err(SandboxError::Timeout {
                    timeout_secs: self.config.timeout_secs,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_builder_works() {
        let config = SandboxConfig::builder()
            .image("ubuntu:22.04")
            .memory("1g")
            .cpus("2.0")
            .timeout(120)
            .network(true)
            .env("FOO", "bar")
            .build();

        assert_eq!(config.image, "ubuntu:22.04");
        assert_eq!(config.memory_limit, "1g");
        assert_eq!(config.cpu_limit, "2.0");
        assert_eq!(config.timeout_secs, 120);
        assert!(config.network_enabled);
        assert_eq!(config.env_vars.len(), 1);
    }

    #[test]
    fn mount_spec_modes() {
        let rw = MountSpec::read_write("/tmp/foo", "/workspace");
        assert!(!rw.read_only);

        let ro = MountSpec::read_only("/tmp/bar", "/data");
        assert!(ro.read_only);
    }

    #[test]
    fn preset_configs() {
        let minimal = SandboxConfig::minimal_resources();
        assert_eq!(minimal.memory_limit, "128m");
        assert_eq!(minimal.timeout_secs, 60);

        let high = SandboxConfig::high_resources();
        assert_eq!(high.memory_limit, "2g");
        assert_eq!(high.timeout_secs, 600);
    }
}
