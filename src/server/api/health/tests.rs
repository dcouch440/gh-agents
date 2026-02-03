//! Tests for health endpoints

use super::*;

#[test]
fn health_response_serializes() {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: "1.0.0".to_string(),
        db_connected: true,
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"status\":\"ok\""));
    assert!(json.contains("\"db_connected\":true"));
}
