//! Project-specific configuration loading

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::types::ProjectConfig;

/// Get the path to the project config file
pub fn project_config_path() -> PathBuf {
    PathBuf::from(".nexor").join("config.toml")
}

/// Load project configuration from .nexor/config.toml
///
/// Returns None if the file doesn't exist (project config is optional).
/// Returns an error if the file exists but cannot be parsed.
pub fn load_project_config() -> Result<Option<ProjectConfig>> {
    let path = project_config_path();

    if !path.exists() {
        tracing::debug!("Project config not found at {:?}, using global only", path);
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read project config from {:?}", path))?;

    let config: ProjectConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse project config from {:?}", path))?;

    tracing::info!("Loaded project config from {:?}", path);
    Ok(Some(config))
}

#[cfg(test)]
mod tests;
