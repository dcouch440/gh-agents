//! Tests for config endpoints

use super::*;
use crate::types::AgentPoolConfig;

#[test]
fn config_response_serializes() {
    let response = ConfigResponse {
        verbosity: "normal".to_string(),
        pool: AgentPoolConfig::default(),
        autonomy: "approval_gates".to_string(),
        git_strategy: "branch_per_slice".to_string(),
        sandbox_mode: "docker".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"verbosity\":\"normal\""));
}
