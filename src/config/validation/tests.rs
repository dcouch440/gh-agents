//! Tests for configuration validation

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
fn zero_agents_fails() {
    let mut config = default_config();
    config.pool.max_agents = 0;
    let result = validate_config(&config);
    assert!(matches!(
        result,
        Err(ConfigValidationError::InvalidPool { .. })
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
