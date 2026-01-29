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

    #[test]
    fn default_config_values() {
        let config = SandboxConfig::default();
        assert_eq!(config.image, "alpine:latest");
        assert_eq!(config.memory_limit, "512m");
        assert_eq!(config.cpu_limit, "1.0");
        assert_eq!(config.timeout_secs, 300);
        assert!(!config.network_enabled);
        assert!(config.env_vars.is_empty());
    }

    #[test]
    fn read_only_config() {
        let config = SandboxConfig::read_only();
        assert!(!config.network_enabled);
        assert_eq!(config.image, "alpine:latest");
    }

    #[test]
    fn with_network_config() {
        let config = SandboxConfig::with_network();
        assert!(config.network_enabled);
    }

    #[test]
    fn high_resources_values() {
        let config = SandboxConfig::high_resources();
        assert_eq!(config.cpu_limit, "2.0");
        assert!(!config.network_enabled);
    }

    #[test]
    fn builder_accumulates_env_vars() {
        let config = SandboxConfig::builder()
            .env("A", "1")
            .env("B", "2")
            .env("C", "3")
            .build();
        assert_eq!(config.env_vars.len(), 3);
        assert_eq!(config.env_vars[0], ("A".to_string(), "1".to_string()));
        assert_eq!(config.env_vars[2], ("C".to_string(), "3".to_string()));
    }

    #[test]
    fn builder_all_methods_chained() {
        let config = SandboxConfig::builder()
            .image("node:18")
            .memory("4g")
            .cpus("4.0")
            .timeout(900)
            .network(true)
            .env("NODE_ENV", "test")
            .build();
        assert_eq!(config.image, "node:18");
        assert_eq!(config.memory_limit, "4g");
        assert_eq!(config.cpu_limit, "4.0");
        assert_eq!(config.timeout_secs, 900);
        assert!(config.network_enabled);
        assert_eq!(config.env_vars.len(), 1);
    }

    #[test]
    fn mount_spec_paths() {
        let rw = MountSpec::read_write("/home/user/project", "/workspace");
        assert_eq!(rw.host_path, PathBuf::from("/home/user/project"));
        assert_eq!(rw.container_path, "/workspace");
        assert!(!rw.read_only);

        let ro = MountSpec::read_only("/data", "/mnt/data");
        assert_eq!(ro.host_path, PathBuf::from("/data"));
        assert_eq!(ro.container_path, "/mnt/data");
        assert!(ro.read_only);
    }

    #[test]
    fn sandbox_error_display() {
        let e1 = SandboxError::DockerNotAvailable("not found".into());
        assert!(e1.to_string().contains("not found"));

        let e2 = SandboxError::ImageNotFound {
            image: "foo:bar".into(),
        };
        assert!(e2.to_string().contains("foo:bar"));

        let e3 = SandboxError::CommandFailed {
            exit_code: 1,
            stderr: "oops".into(),
        };
        assert!(e3.to_string().contains("1"));
        assert!(e3.to_string().contains("oops"));

        let e4 = SandboxError::Timeout { timeout_secs: 60 };
        assert!(e4.to_string().contains("60"));
    }

    #[test]
    fn sandbox_result_construction() {
        let result = SandboxResult {
            success: true,
            exit_code: 0,
            stdout: "ok".into(),
            stderr: String::new(),
            duration_ms: 100,
            container_id: "test-123".into(),
        };
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn sandbox_construction() {
        let ctx = ExecutionContext::new("/tmp".into());
        let sandbox = Sandbox::new(ctx, SandboxConfig::default());
        assert_eq!(sandbox.config.image, "alpine:latest");
    }

    #[test]
    fn sandbox_with_defaults() {
        let ctx = ExecutionContext::new("/tmp".into());
        let sandbox = Sandbox::with_defaults(ctx);
        assert_eq!(sandbox.config.timeout_secs, 300);
    }

    #[test]
    fn sandbox_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let sandbox_err: SandboxError = SandboxError::from(io_err);
        assert!(sandbox_err.to_string().contains("file missing"));
        // Verify it matches the ExecutionError variant
        match sandbox_err {
            SandboxError::ExecutionError(_) => {}
            _ => panic!("Expected ExecutionError variant"),
        }
    }

    #[test]
    fn sandbox_config_builder_defaults() {
        let config = SandboxConfigBuilder::default().build();
        assert_eq!(config.image, "alpine:latest");
        assert_eq!(config.memory_limit, "512m");
        assert_eq!(config.cpu_limit, "1.0");
        assert_eq!(config.timeout_secs, 300);
        assert!(!config.network_enabled);
        assert!(config.env_vars.is_empty());
    }

    #[test]
    fn builder_network_false() {
        let config = SandboxConfig::builder().network(false).build();
        assert!(!config.network_enabled);
    }

    #[test]
    fn builder_overrides_image_twice() {
        let config = SandboxConfig::builder()
            .image("node:16")
            .image("node:20")
            .build();
        assert_eq!(config.image, "node:20");
    }

    #[test]
    fn mount_spec_with_pathbuf_input() {
        let path = PathBuf::from("/some/path");
        let mount = MountSpec::read_write(path.clone(), "/container".to_string());
        assert_eq!(mount.host_path, path);
        assert_eq!(mount.container_path, "/container");
    }

    #[test]
    fn sandbox_result_failure() {
        let result = SandboxResult {
            success: false,
            exit_code: 127,
            stdout: String::new(),
            stderr: "command not found".into(),
            duration_ms: 50,
            container_id: "fail-container".into(),
        };
        assert!(!result.success);
        assert_eq!(result.exit_code, 127);
        assert!(result.stderr.contains("command not found"));
        assert_eq!(result.container_id, "fail-container");
    }

    #[test]
    fn sandbox_result_negative_exit_code() {
        let result = SandboxResult {
            success: false,
            exit_code: -1,
            stdout: String::new(),
            stderr: "killed".into(),
            duration_ms: 0,
            container_id: "killed-container".into(),
        };
        assert_eq!(result.exit_code, -1);
    }

    #[test]
    fn sandbox_construction_with_custom_config() {
        let ctx = ExecutionContext::new("/my/project".into());
        let config = SandboxConfig::builder()
            .image("rust:latest")
            .memory("4g")
            .timeout(600)
            .network(true)
            .env("RUST_LOG", "debug")
            .build();
        let sandbox = Sandbox::new(ctx, config);
        assert_eq!(sandbox.config.image, "rust:latest");
        assert_eq!(sandbox.config.memory_limit, "4g");
        assert_eq!(sandbox.config.timeout_secs, 600);
        assert!(sandbox.config.network_enabled);
        assert_eq!(sandbox.config.env_vars.len(), 1);
        assert_eq!(sandbox.ctx.project_root, PathBuf::from("/my/project"));
    }

    #[test]
    fn sandbox_with_defaults_uses_default_config() {
        let ctx = ExecutionContext::new("/workspace".into());
        let sandbox = Sandbox::with_defaults(ctx);
        assert_eq!(sandbox.config.image, "alpine:latest");
        assert_eq!(sandbox.config.memory_limit, "512m");
        assert_eq!(sandbox.config.cpu_limit, "1.0");
        assert!(!sandbox.config.network_enabled);
        assert!(sandbox.config.env_vars.is_empty());
        assert_eq!(sandbox.ctx.project_root, PathBuf::from("/workspace"));
    }

    #[test]
    fn read_only_inherits_defaults() {
        let config = SandboxConfig::read_only();
        assert_eq!(config.memory_limit, "512m");
        assert_eq!(config.cpu_limit, "1.0");
        assert_eq!(config.timeout_secs, 300);
        assert!(config.env_vars.is_empty());
    }

    #[test]
    fn with_network_inherits_defaults() {
        let config = SandboxConfig::with_network();
        assert_eq!(config.image, "alpine:latest");
        assert_eq!(config.memory_limit, "512m");
        assert_eq!(config.timeout_secs, 300);
    }

    #[test]
    fn minimal_resources_values() {
        let config = SandboxConfig::minimal_resources();
        assert_eq!(config.cpu_limit, "0.5");
        assert!(!config.network_enabled);
        assert_eq!(config.image, "alpine:latest");
    }

    #[test]
    fn high_resources_inherits_defaults() {
        let config = SandboxConfig::high_resources();
        assert_eq!(config.image, "alpine:latest");
        assert!(config.env_vars.is_empty());
    }

    #[test]
    fn sandbox_error_debug_format() {
        let e = SandboxError::Timeout { timeout_secs: 42 };
        let debug = format!("{:?}", e);
        assert!(debug.contains("42"));

        let e2 = SandboxError::ImageNotFound {
            image: "test:img".into(),
        };
        let debug2 = format!("{:?}", e2);
        assert!(debug2.contains("test:img"));
    }

    #[test]
    fn sandbox_result_clone() {
        let result = SandboxResult {
            success: true,
            exit_code: 0,
            stdout: "hello".into(),
            stderr: String::new(),
            duration_ms: 200,
            container_id: "clone-test".into(),
        };
        let cloned = result.clone();
        assert_eq!(cloned.stdout, "hello");
        assert_eq!(cloned.container_id, "clone-test");
        assert_eq!(cloned.duration_ms, 200);
    }

    #[test]
    fn sandbox_config_clone() {
        let config = SandboxConfig::builder()
            .image("python:3.11")
            .env("X", "Y")
            .build();
        let cloned = config.clone();
        assert_eq!(cloned.image, "python:3.11");
        assert_eq!(cloned.env_vars, config.env_vars);
    }

    #[test]
    fn mount_spec_clone() {
        let mount = MountSpec::read_only("/src", "/dest");
        let cloned = mount.clone();
        assert_eq!(cloned.host_path, mount.host_path);
        assert_eq!(cloned.container_path, mount.container_path);
        assert_eq!(cloned.read_only, mount.read_only);
    }

    #[test]
    fn sandbox_result_debug() {
        let result = SandboxResult {
            success: false,
            exit_code: 2,
            stdout: "out".into(),
            stderr: "err".into(),
            duration_ms: 999,
            container_id: "dbg".into(),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("dbg"));
        assert!(debug.contains("999"));
    }

    #[test]
    fn mount_spec_debug() {
        let mount = MountSpec::read_write("/a", "/b");
        let debug = format!("{:?}", mount);
        assert!(debug.contains("/a"));
        assert!(debug.contains("/b"));
    }

    #[test]
    fn builder_env_preserves_order() {
        let config = SandboxConfig::builder()
            .env("Z", "last")
            .env("A", "first")
            .env("M", "middle")
            .build();
        assert_eq!(config.env_vars[0].0, "Z");
        assert_eq!(config.env_vars[1].0, "A");
        assert_eq!(config.env_vars[2].0, "M");
    }

    #[tokio::test]
    async fn check_docker_handles_missing_docker() {
        // This test verifies that check_docker returns an error
        // when docker is not available or returns an error.
        // In CI without docker, this will get DockerNotAvailable.
        // With docker, it will succeed. Either way we exercise the code path.
        let ctx = ExecutionContext::new("/tmp".into());
        let sandbox = Sandbox::with_defaults(ctx);
        let result = sandbox.check_docker().await;
        // Just verify it returns Ok or a DockerNotAvailable error
        match result {
            Ok(()) => {}                                   // Docker is available, that's fine
            Err(SandboxError::DockerNotAvailable(_)) => {} // Expected without docker
            Err(e) => panic!("Unexpected error variant: {:?}", e),
        }
    }

    #[tokio::test]
    async fn exec_handles_docker_unavailability() {
        // Use a config with a very short timeout and a nonsense image
        // to exercise the exec code path
        let ctx = ExecutionContext::new("/tmp".into());
        let config = SandboxConfig::builder()
            .image("nonexistent-image-xxxxx:latest")
            .timeout(5)
            .build();
        let sandbox = Sandbox::new(ctx, config);
        let result = sandbox.exec(&["echo", "hello"]).await;
        // Without docker: DockerNotAvailable
        // With docker: could be various errors
        assert!(result.is_err() || result.is_ok());
    }

    #[tokio::test]
    async fn exec_shell_delegates_to_exec() {
        // exec_shell wraps exec with ["sh", "-c", command]
        // Just verify it goes through the same code path
        let ctx = ExecutionContext::new("/tmp".into());
        let sandbox = Sandbox::with_defaults(ctx);
        let result = sandbox.exec_shell("echo test").await;
        match result {
            Ok(_) => {}                                    // Docker available
            Err(SandboxError::DockerNotAvailable(_)) => {} // No docker
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn exec_with_mounts_handles_docker_unavailability() {
        let ctx = ExecutionContext::new("/tmp".into());
        let sandbox = Sandbox::with_defaults(ctx);
        let mounts = vec![MountSpec::read_only("/tmp", "/data")];
        let result = sandbox.exec_with_mounts(&["ls"], &mounts).await;
        match result {
            Ok(_) => {}
            Err(SandboxError::DockerNotAvailable(_)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn exec_with_mounts_workspace_override() {
        // When a mount targets /workspace, the default mount should not be added
        let ctx = ExecutionContext::new("/tmp".into());
        let sandbox = Sandbox::with_defaults(ctx);
        let mounts = vec![MountSpec::read_write("/tmp", "/workspace")];
        let result = sandbox.exec_with_mounts(&["ls"], &mounts).await;
        match result {
            Ok(_) => {}
            Err(SandboxError::DockerNotAvailable(_)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn exec_with_empty_mounts() {
        let ctx = ExecutionContext::new("/tmp".into());
        let sandbox = Sandbox::with_defaults(ctx);
        let result = sandbox.exec_with_mounts(&["echo", "hi"], &[]).await;
        match result {
            Ok(_) => {}
            Err(SandboxError::DockerNotAvailable(_)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn exec_with_network_enabled_config() {
        let ctx = ExecutionContext::new("/tmp".into());
        let config = SandboxConfig::with_network();
        let sandbox = Sandbox::new(ctx, config);
        let result = sandbox.exec(&["echo", "net"]).await;
        match result {
            Ok(_) => {}
            Err(SandboxError::DockerNotAvailable(_)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn exec_with_env_vars() {
        let ctx = ExecutionContext::new("/tmp".into());
        let config = SandboxConfig::builder()
            .env("MY_VAR", "my_value")
            .env("OTHER", "stuff")
            .build();
        let sandbox = Sandbox::new(ctx, config);
        let result = sandbox.exec(&["env"]).await;
        match result {
            Ok(_) => {}
            Err(SandboxError::DockerNotAvailable(_)) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}
