//! Configuration loading and management

mod global;
mod project;
mod validation;

pub use global::{global_config_path, load_global_config};
pub use project::{load_project_config, project_config_path};
pub use validation::{validate_config, ConfigValidationError};

use crate::types::AppConfig;
use anyhow::Result;

/// Load, merge, and validate all configuration sources
pub fn load_config() -> Result<AppConfig> {
    let global = load_global_config()?;
    let project = load_project_config()?;
    let config = AppConfig::merge(global, project);

    validate_config(&config)?;

    Ok(config)
}
