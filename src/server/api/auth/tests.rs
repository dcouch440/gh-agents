//! Tests for auth endpoints

use super::*;

#[test]
fn setup_request_deserializes() {
    let json = r#"{"password":"mypassword"}"#;
    let request: SetupRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.password, "mypassword");
}

#[test]
fn login_request_deserializes() {
    let json = r#"{"email":"test@test.com","password":"mypassword"}"#;
    let request: LoginRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.email, "test@test.com");
    assert_eq!(request.password, "mypassword");
}

#[test]
fn setup_response_serializes() {
    let response = SetupResponse { message: "ok".to_string() };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"message\":\"ok\""));
}

#[test]
fn login_response_serializes() {
    let response = LoginResponse {
        token: "abc123".to_string(),
        expires_in: 86400,
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"token\":\"abc123\""));
    assert!(json.contains("\"expires_in\":86400"));
}

#[test]
fn me_response_serializes() {
    let response = MeResponse {
        id: "user-123".to_string(),
        email: "admin@example.com".to_string(),
        github_login: None,
        authenticated: true,
        token_expires: 99999,
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"id\":\"user-123\""));
    assert!(json.contains("\"authenticated\":true"));
    assert!(json.contains("\"token_expires\":99999"));
}
