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

    let content = std::fs::read_to_string(&path).with_context(|| format!("Failed to read global config from {:?}", path))?;

    let config: GlobalConfig = toml::from_str(&content).with_context(|| format!("Failed to parse global config from {:?}", path))?;

    tracing::info!("Loaded global config from {:?}", path);
    Ok(config)
}

#[cfg(test)]
mod tests;
