//! Configuration validation

use thiserror::Error;

use crate::types::AppConfig;

/// Errors that can occur during config validation
#[derive(Error, Debug)]
pub enum ConfigValidationError {
    #[error("Invalid pool configuration: {reason}")]
    InvalidPool { reason: String },

    #[error("Invalid model configuration: {reason}")]
    InvalidModel { reason: String },

    #[error("Conflicting configuration: {reason}")]
    Conflict { reason: String },
}

/// Validate a merged configuration
pub fn validate_config(config: &AppConfig) -> Result<(), ConfigValidationError> {
    validate_pool_config(config)?;
    validate_model_config(config)?;
    validate_consistency(config)?;
    Ok(())
}

fn validate_pool_config(config: &AppConfig) -> Result<(), ConfigValidationError> {
    if config.pool.max_orchestrators == 0 {
        return Err(ConfigValidationError::InvalidPool {
            reason: "max_orchestrators must be at least 1".to_string(),
        });
    }

    if config.pool.max_workers == 0 {
        return Err(ConfigValidationError::InvalidPool {
            reason: "max_workers must be at least 1".to_string(),
        });
    }

    // Utilities can be 0 (optional)

    Ok(())
}

fn validate_model_config(config: &AppConfig) -> Result<(), ConfigValidationError> {
    if config.models.orchestrator.max_tokens == 0 {
        return Err(ConfigValidationError::InvalidModel {
            reason: "orchestrator max_tokens must be greater than 0".to_string(),
        });
    }

    if config.models.worker.max_tokens == 0 {
        return Err(ConfigValidationError::InvalidModel {
            reason: "worker max_tokens must be greater than 0".to_string(),
        });
    }

    if config.models.utility.max_tokens == 0 {
        return Err(ConfigValidationError::InvalidModel {
            reason: "utility max_tokens must be greater than 0".to_string(),
        });
    }

    if config.models.orchestrator.model_id.is_empty() {
        return Err(ConfigValidationError::InvalidModel {
            reason: "orchestrator model_id cannot be empty".to_string(),
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
mod tests {
    use super::*;
    use crate::types::{AppConfig, GlobalConfig};

    fn default_config() -> AppConfig {
        AppConfig::merge(GlobalConfig::default(), None)
    }

    #[test]
    fn valid_config_passes() {
        let config = default_config();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn zero_orchestrators_fails() {
        let mut config = default_config();
        config.pool.max_orchestrators = 0;
        let result = validate_config(&config);
        assert!(matches!(
            result,
            Err(ConfigValidationError::InvalidPool { .. })
        ));
    }

    #[test]
    fn zero_workers_fails() {
        let mut config = default_config();
        config.pool.max_workers = 0;
        let result = validate_config(&config);
        assert!(matches!(
            result,
            Err(ConfigValidationError::InvalidPool { .. })
        ));
    }

    #[test]
    fn empty_model_id_fails() {
        let mut config = default_config();
        config.models.orchestrator.model_id = String::new();
        let result = validate_config(&config);
        assert!(matches!(
            result,
            Err(ConfigValidationError::InvalidModel { .. })
        ));
    }

    #[test]
    fn full_auto_with_gates_fails() {
        let mut config = default_config();
        config.autonomy = crate::types::AutonomyLevel::FullAuto;
        config.approval_gates.before_pr = true;
        let result = validate_config(&config);
        assert!(matches!(
            result,
            Err(ConfigValidationError::Conflict { .. })
        ));
    }
}
