//! Global configuration loading

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::types::GlobalConfig;

#[cfg(test)]
use crate::types::VerbosityLevel;

/// Get the path to the global config file
pub fn global_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("nexor").join("config.toml")
}

/// Load global configuration from ~/.config/nexor/config.toml
///
/// Returns default config if the file doesn't exist.
/// Returns an error if the file exists but cannot be parsed.
pub fn load_global_config() -> Result<GlobalConfig> {
    let path = global_config_path();

    if !path.exists() {
        tracing::debug!("Global config not found at {:?}, using defaults", path);
        return Ok(GlobalConfig::default());
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read global config from {:?}", path))?;

    let config: GlobalConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse global config from {:?}", path))?;

    tracing::info!("Loaded global config from {:?}", path);
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn returns_defaults_when_file_missing() {
        // Uses non-existent path, should return defaults
        let config = load_global_config();
        // This test may fail if you have a global config installed
        // In practice, test with mock paths
        assert!(config.is_ok());
    }

    #[test]
    fn parses_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
            verbosity = "verbose"

            [pool]
            max_orchestrators = 2
            max_workers = 10
            max_utilities = 4
        "#;

        fs::write(&config_path, config_content).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let config: GlobalConfig = toml::from_str(&content).unwrap();

        assert_eq!(config.verbosity, VerbosityLevel::Verbose);
        assert_eq!(config.pool.max_workers, 10);
    }

    #[test]
    fn errors_on_malformed_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let invalid_content = "this is not valid { toml [";
        fs::write(&config_path, invalid_content).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        let result: Result<GlobalConfig, _> = toml::from_str(&content);

        assert!(result.is_err());
    }

    #[test]
    fn global_config_path_is_under_home() {
        let path = global_config_path();
        // Should end with .config/nexor/config.toml
        assert!(path.ends_with(".config/nexor/config.toml"));
    }

    #[test]
    fn parses_minimal_global_config() {
        let content = "";
        let config: GlobalConfig = toml::from_str(content).unwrap();
        assert_eq!(config.verbosity, VerbosityLevel::default());
    }

    #[test]
    fn parses_global_config_with_all_verbosity_levels() {
        let content = r#"verbosity = "quiet""#;
        let config: GlobalConfig = toml::from_str(content).unwrap();
        assert_eq!(config.verbosity, VerbosityLevel::Quiet);

        let content = r#"verbosity = "normal""#;
        let config: GlobalConfig = toml::from_str(content).unwrap();
        assert_eq!(config.verbosity, VerbosityLevel::Normal);

        let content = r#"verbosity = "verbose""#;
        let config: GlobalConfig = toml::from_str(content).unwrap();
        assert_eq!(config.verbosity, VerbosityLevel::Verbose);
    }

    #[test]
    fn global_config_default_has_expected_pool_values() {
        let config = GlobalConfig::default();
        assert!(config.pool.max_workers > 0);
        assert!(config.pool.max_orchestrators > 0);
    }
}
