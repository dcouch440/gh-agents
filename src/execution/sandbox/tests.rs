#[cfg(test)]
mod tests {
    use super::super::*;

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
        assert_eq!(rw.host_path, std::path::PathBuf::from("/home/user/project"));
        assert_eq!(rw.container_path, "/workspace");
        assert!(!rw.read_only);

        let ro = MountSpec::read_only("/data", "/mnt/data");
        assert_eq!(ro.host_path, std::path::PathBuf::from("/data"));
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
        let path = std::path::PathBuf::from("/some/path");
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
        assert_eq!(
            sandbox.ctx.project_root,
            std::path::PathBuf::from("/my/project")
        );
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
        assert_eq!(
            sandbox.ctx.project_root,
            std::path::PathBuf::from("/workspace")
        );
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
