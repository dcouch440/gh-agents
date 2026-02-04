//! Configuration validation

use thiserror::Error;

use crate::types::AppConfig;

/// Errors that can occur during config validation
#[derive(Error, Debug)]
pub enum ConfigValidationError {
    #[error("Invalid pool configuration: {reason}")]
    InvalidPool { reason: String },

    #[error("Conflicting configuration: {reason}")]
    Conflict { reason: String },
}

/// Validate a merged configuration
pub fn validate_config(config: &AppConfig) -> Result<(), ConfigValidationError> {
    validate_pool_config(config)?;
    validate_consistency(config)?;
    Ok(())
}

fn validate_pool_config(config: &AppConfig) -> Result<(), ConfigValidationError> {
    if config.pool.max_agents == 0 {
        return Err(ConfigValidationError::InvalidPool {
            reason: "max_agents must be at least 1".to_string(),
        });
    }

    Ok(())
}

fn validate_consistency(config: &AppConfig) -> Result<(), ConfigValidationError> {
    // FullAuto mode should not have approval gates that would block
    if config.autonomy == crate::types::AutonomyLevel::FullAuto
        && (config.approval_gates.before_commit
            || config.approval_gates.before_pr
            || config.approval_gates.before_merge)
    {
        return Err(ConfigValidationError::Conflict {
            reason: "FullAuto autonomy conflicts with enabled approval gates".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests;
