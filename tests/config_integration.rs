//! Integration tests for configuration loading and merging

use nexor::types::{AppConfig, AutonomyLevel, GlobalConfig, ProjectConfig, SandboxMode};

#[test]
fn merge_uses_project_overrides() {
    let global = GlobalConfig::default();
    let project = ProjectConfig {
        autonomy: AutonomyLevel::FullAuto,
        sandbox_mode: SandboxMode::None,
        ..Default::default()
    };

    let merged = AppConfig::merge(global, Some(project));

    assert_eq!(merged.autonomy, AutonomyLevel::FullAuto);
    assert_eq!(merged.sandbox_mode, SandboxMode::None);
}

#[test]
fn merge_uses_global_when_no_project() {
    let global = GlobalConfig::default();
    let merged = AppConfig::merge(global.clone(), None);

    assert_eq!(merged.models, global.default_models);
    assert_eq!(merged.pool, global.pool);
}

#[test]
fn merge_partial_project_config() {
    let global = GlobalConfig::default();
    let project = ProjectConfig {
        autonomy: AutonomyLevel::Supervised,
        // models is None, should use global
        ..Default::default()
    };

    let merged = AppConfig::merge(global.clone(), Some(project));

    assert_eq!(merged.autonomy, AutonomyLevel::Supervised);
    assert_eq!(merged.models, global.default_models); // From global
}
