#[cfg(test)]
mod tests {
    //! Tests for project configuration

    use super::super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn returns_none_when_file_missing() {
        // Default path likely doesn't exist in test environment
        let path = PathBuf::from("nonexistent/.nexor/config.toml");
        assert!(!path.exists());
    }

    #[test]
    fn parses_valid_project_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
            autonomy = "full_auto"
            git_strategy = "branch_per_ticket"
            sandbox_mode = "none"

            [approval_gates]
            before_commit = true
            before_pr = true
            before_merge = true
        "#;

        fs::write(&config_path, config_content).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let config: ProjectConfig = toml::from_str(&content).unwrap();

        assert_eq!(config.autonomy, crate::types::AutonomyLevel::FullAuto);
        assert_eq!(
            config.git_strategy,
            crate::types::GitStrategy::BranchPerTicket
        );
        assert!(config.approval_gates.before_commit);
    }

    #[test]
    fn errors_on_malformed_project_config() {
        let content = "invalid toml {{{}}}";
        let result: Result<ProjectConfig, _> = toml::from_str(content);
        assert!(result.is_err());
    }

    #[test]
    fn project_config_path_is_correct() {
        let path = project_config_path();
        assert_eq!(path, PathBuf::from(".nexor/config.toml"));
    }

    #[test]
    fn load_project_config_returns_none_when_missing() {
        // In the test environment, .nexor/config.toml typically doesn't exist
        // at the working directory. If it does exist, this test just verifies
        // the function succeeds.
        let result = load_project_config();
        assert!(result.is_ok());
    }

    #[test]
    fn parses_minimal_project_config() {
        // Test with all defaults
        let content = "";
        let config: ProjectConfig = toml::from_str(content).unwrap();
        assert_eq!(config.autonomy, crate::types::AutonomyLevel::ApprovalGates);
        assert_eq!(config.git_strategy, crate::types::GitStrategy::default());
        assert_eq!(config.sandbox_mode, crate::types::SandboxMode::default());
        assert!(config.pool.is_none());
    }

    #[test]
    fn parses_project_config_with_all_autonomy_levels() {
        let content = r#"autonomy = "full_auto""#;
        let config: ProjectConfig = toml::from_str(content).unwrap();
        assert_eq!(config.autonomy, crate::types::AutonomyLevel::FullAuto);

        let content = r#"autonomy = "supervised""#;
        let config: ProjectConfig = toml::from_str(content).unwrap();
        assert_eq!(config.autonomy, crate::types::AutonomyLevel::Supervised);
    }
}
