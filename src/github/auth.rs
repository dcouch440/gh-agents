//! GitHub Device Flow authentication

use reqwest::Client;
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("device flow error: {error} - {error_description}")]
    DeviceFlowError {
        error: String,
        error_description: String,
    },

    #[error("authorization timed out - user did not complete flow")]
    Timeout,

    #[error("authorization denied by user")]
    AccessDenied,

    #[error("token expired, please re-authenticate")]
    TokenExpired,

    #[error("failed to store credentials: {0}")]
    StorageError(String),

    #[error("no credentials found - run 'nexor auth login' first")]
    NotAuthenticated,
}

/// GitHub OAuth App client ID
/// Users can override this with their own OAuth app for self-hosted scenarios
const DEFAULT_CLIENT_ID: &str = "Ov23liY9WN0fxkU5Ijze"; // nexor OAuth app

/// OAuth scopes needed for nexor
const SCOPES: &str = "repo read:org";

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum TokenResponse {
    Success {
        access_token: String,
        #[allow(dead_code)]
        token_type: String,
        #[allow(dead_code)]
        scope: String,
    },
    Error {
        error: String,
        error_description: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
    pub name: Option<String>,
    pub email: Option<String>,
}

pub struct GitHubAuth {
    client: Client,
    client_id: String,
    api_base_url: String,
}

impl GitHubAuth {
    pub fn new() -> Self {
        Self::with_client_id(DEFAULT_CLIENT_ID.to_string())
    }

    pub fn with_client_id(client_id: String) -> Self {
        let client = Client::builder()
            .user_agent("nexor")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            client_id,
            api_base_url: "https://api.github.com".to_string(),
        }
    }

    /// Set a custom API base URL (useful for testing)
    #[cfg(test)]
    pub fn with_api_base_url(mut self, url: String) -> Self {
        self.api_base_url = url;
        self
    }

    /// Step 1: Request a device code from GitHub
    pub async fn request_device_code(&self) -> Result<DeviceCodeResponse, AuthError> {
        let response = self
            .client
            .post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", &self.client_id),
                ("scope", &SCOPES.to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AuthError::DeviceFlowError {
                error: "request_failed".to_string(),
                error_description: text,
            });
        }

        let device_code: DeviceCodeResponse = response.json().await?;

        tracing::info!(
            user_code = %device_code.user_code,
            verification_uri = %device_code.verification_uri,
            expires_in = device_code.expires_in,
            "Device code received"
        );

        Ok(device_code)
    }

    /// Step 2: Poll for access token until user completes auth
    pub async fn poll_for_token(
        &self,
        device_code: &DeviceCodeResponse,
    ) -> Result<String, AuthError> {
        let mut interval = Duration::from_secs(device_code.interval);
        let deadline = std::time::Instant::now() + Duration::from_secs(device_code.expires_in);

        loop {
            // Check if we've exceeded the deadline
            if std::time::Instant::now() > deadline {
                return Err(AuthError::Timeout);
            }

            // Wait before polling
            tokio::time::sleep(interval).await;

            // Poll for token
            let response = self
                .client
                .post("https://github.com/login/oauth/access_token")
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", &self.client_id),
                    ("device_code", &device_code.device_code),
                    (
                        "grant_type",
                        &"urn:ietf:params:oauth:grant-type:device_code".to_string(),
                    ),
                ])
                .send()
                .await?;

            let token_response: TokenResponse = response.json().await?;

            match token_response {
                TokenResponse::Success { access_token, .. } => {
                    tracing::info!("Successfully authenticated with GitHub");
                    return Ok(access_token);
                }
                TokenResponse::Error {
                    error,
                    error_description,
                } => match error.as_str() {
                    "authorization_pending" => {
                        // User hasn't completed auth yet, keep polling
                        tracing::debug!("Authorization pending, continuing to poll...");
                        continue;
                    }
                    "slow_down" => {
                        // We're polling too fast, increase interval
                        interval += Duration::from_secs(5);
                        tracing::debug!(interval_secs = interval.as_secs(), "Slowing down polling");
                        continue;
                    }
                    "expired_token" => {
                        return Err(AuthError::Timeout);
                    }
                    "access_denied" => {
                        return Err(AuthError::AccessDenied);
                    }
                    _ => {
                        return Err(AuthError::DeviceFlowError {
                            error,
                            error_description: error_description.unwrap_or_default(),
                        });
                    }
                },
            }
        }
    }

    /// Complete device flow: request code, display to user, poll for token
    pub async fn device_flow_login<F>(&self, display_fn: F) -> Result<String, AuthError>
    where
        F: FnOnce(&str, &str), // (user_code, verification_uri)
    {
        // Step 1: Get device code
        let device_code = self.request_device_code().await?;

        // Step 2: Display code to user
        display_fn(&device_code.user_code, &device_code.verification_uri);

        // Step 3: Poll for token
        let token = self.poll_for_token(&device_code).await?;

        Ok(token)
    }

    /// Configure git to use the stored GitHub token for authentication
    pub fn configure_git_credentials(token: &str) -> Result<(), AuthError> {
        // Configure git credential helper for github.com
        // The helper echoes the token for HTTPS authentication
        let helper_command = format!(
            r#"!f() {{ test "$1" = get && echo "protocol=https" && echo "host=github.com" && echo "username=x-access-token" && echo "password={}"; }}; f"#,
            token
        );

        // Set the credential helper for github.com only
        let output = Command::new("git")
            .args([
                "config",
                "--global",
                "credential.https://github.com.helper",
                &helper_command,
            ])
            .output()
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AuthError::StorageError(format!(
                "Failed to configure git credentials: {}",
                stderr
            )));
        }

        tracing::info!("Configured git credentials for github.com");
        Ok(())
    }

    /// Remove git credential configuration
    pub fn remove_git_credentials() -> Result<(), AuthError> {
        let _ = Command::new("git")
            .args([
                "config",
                "--global",
                "--unset",
                "credential.https://github.com.helper",
            ])
            .output();

        tracing::info!("Removed git credentials for github.com");
        Ok(())
    }

    /// Verify the token works by calling GitHub API
    pub async fn verify_token(&self, token: &str) -> Result<GitHubUser, AuthError> {
        let response = self
            .client
            .get(format!("{}/user", self.api_base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "nexor")
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(AuthError::TokenExpired);
        }

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AuthError::DeviceFlowError {
                error: "verification_failed".to_string(),
                error_description: text,
            });
        }

        let user: GitHubUser = response.json().await?;
        tracing::info!(login = %user.login, "Token verified");

        Ok(user)
    }

    /// Open browser to verification URL
    pub fn open_browser(url: &str) -> Result<(), std::io::Error> {
        #[cfg(target_os = "macos")]
        {
            Command::new("open").arg(url).spawn()?;
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open").arg(url).spawn()?;
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("cmd").args(["/C", "start", url]).spawn()?;
        }

        Ok(())
    }
}

impl Default for GitHubAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scopes_include_repo() {
        assert!(SCOPES.contains("repo"));
        assert!(SCOPES.contains("read:org"));
    }

    #[test]
    fn auth_creates_with_client_id() {
        let auth = GitHubAuth::with_client_id("test-client-id".to_string());
        assert_eq!(auth.client_id, "test-client-id");
    }

    #[test]
    fn default_uses_default_client_id() {
        let auth = GitHubAuth::new();
        assert_eq!(auth.client_id, DEFAULT_CLIENT_ID);
    }

    #[tokio::test]
    async fn verify_token_returns_user_on_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "login": "testuser",
                "id": 12345,
                "name": "Test User",
                "email": "test@example.com"
            })))
            .mount(&mock_server)
            .await;

        let auth = GitHubAuth::with_client_id("test-id".to_string())
            .with_api_base_url(mock_server.uri());

        let user = auth.verify_token("fake-token").await.unwrap();
        assert_eq!(user.login, "testuser");
        assert_eq!(user.id, 12345);
        assert_eq!(user.name, Some("Test User".to_string()));
        assert_eq!(user.email, Some("test@example.com".to_string()));
    }

    #[tokio::test]
    async fn verify_token_returns_token_expired_on_401() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let auth = GitHubAuth::with_client_id("test-id".to_string())
            .with_api_base_url(mock_server.uri());

        let err = auth.verify_token("bad-token").await.unwrap_err();
        assert!(
            matches!(err, AuthError::TokenExpired),
            "Expected TokenExpired, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn verify_token_returns_device_flow_error_on_500() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .mount(&mock_server)
            .await;

        let auth = GitHubAuth::with_client_id("test-id".to_string())
            .with_api_base_url(mock_server.uri());

        let err = auth.verify_token("fake-token").await.unwrap_err();
        match err {
            AuthError::DeviceFlowError {
                error,
                error_description,
            } => {
                assert_eq!(error, "verification_failed");
                assert_eq!(error_description, "internal error");
            }
            other => panic!("Expected DeviceFlowError, got: {other:?}"),
        }
    }

    #[test]
    fn default_impl_uses_default_client_id() {
        let auth = GitHubAuth::default();
        assert_eq!(auth.client_id, DEFAULT_CLIENT_ID);
        assert_eq!(auth.api_base_url, "https://api.github.com");
    }

    #[test]
    fn with_api_base_url_overrides_default() {
        let auth = GitHubAuth::with_client_id("id".to_string())
            .with_api_base_url("http://localhost:9999".to_string());
        assert_eq!(auth.api_base_url, "http://localhost:9999");
    }

    #[test]
    fn auth_error_display_messages() {
        let err = AuthError::Timeout;
        assert_eq!(
            err.to_string(),
            "authorization timed out - user did not complete flow"
        );

        let err = AuthError::AccessDenied;
        assert_eq!(err.to_string(), "authorization denied by user");

        let err = AuthError::TokenExpired;
        assert_eq!(err.to_string(), "token expired, please re-authenticate");

        let err = AuthError::StorageError("disk full".to_string());
        assert_eq!(err.to_string(), "failed to store credentials: disk full");

        let err = AuthError::NotAuthenticated;
        assert_eq!(
            err.to_string(),
            "no credentials found - run 'nexor auth login' first"
        );

        let err = AuthError::DeviceFlowError {
            error: "bad_code".to_string(),
            error_description: "invalid device code".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "device flow error: bad_code - invalid device code"
        );
    }

    #[test]
    fn remove_git_credentials_succeeds() {
        // remove_git_credentials always returns Ok even if nothing to unset
        let result = GitHubAuth::remove_git_credentials();
        assert!(result.is_ok());
    }

    #[test]
    fn open_browser_does_not_panic() {
        // We can't fully test browser opening, but we can verify it doesn't panic
        // with an invalid URL (the spawn may fail but that's an Err, not a panic)
        let _result = GitHubAuth::open_browser("http://example.com");
    }

    #[tokio::test]
    async fn verify_token_user_with_null_optional_fields() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "login": "minimaluser",
                "id": 1,
                "name": null,
                "email": null
            })))
            .mount(&mock_server)
            .await;

        let auth = GitHubAuth::with_client_id("test-id".to_string())
            .with_api_base_url(mock_server.uri());

        let user = auth.verify_token("token").await.unwrap();
        assert_eq!(user.login, "minimaluser");
        assert_eq!(user.id, 1);
        assert!(user.name.is_none());
        assert!(user.email.is_none());
    }

    #[tokio::test]
    async fn verify_token_returns_error_on_non_json_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&mock_server)
            .await;

        let auth = GitHubAuth::with_client_id("test-id".to_string())
            .with_api_base_url(mock_server.uri());

        let err = auth.verify_token("token").await;
        assert!(err.is_err());
        assert!(matches!(err.unwrap_err(), AuthError::RequestFailed(_)));
    }

    #[tokio::test]
    async fn verify_token_sends_correct_auth_header() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/user"))
            .and(header("Authorization", "Bearer my-secret-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "login": "headeruser",
                "id": 99,
                "name": null,
                "email": null
            })))
            .mount(&mock_server)
            .await;

        let auth = GitHubAuth::with_client_id("test-id".to_string())
            .with_api_base_url(mock_server.uri());

        let user = auth.verify_token("my-secret-token").await.unwrap();
        assert_eq!(user.login, "headeruser");
    }

    #[test]
    fn token_response_deserializes_success() {
        let json = r#"{"access_token":"gho_abc","token_type":"bearer","scope":"repo"}"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        match resp {
            TokenResponse::Success { access_token, .. } => {
                assert_eq!(access_token, "gho_abc");
            }
            _ => panic!("Expected Success variant"),
        }
    }

    #[test]
    fn token_response_deserializes_error() {
        let json = r#"{"error":"authorization_pending","error_description":"waiting"}"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        match resp {
            TokenResponse::Error {
                error,
                error_description,
            } => {
                assert_eq!(error, "authorization_pending");
                assert_eq!(error_description, Some("waiting".to_string()));
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn token_response_deserializes_error_without_description() {
        let json = r#"{"error":"access_denied"}"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        match resp {
            TokenResponse::Error {
                error,
                error_description,
            } => {
                assert_eq!(error, "access_denied");
                assert!(error_description.is_none());
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn device_code_response_deserializes() {
        let json = r#"{
            "device_code": "dc_123",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        }"#;
        let resp: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.device_code, "dc_123");
        assert_eq!(resp.user_code, "ABCD-1234");
        assert_eq!(resp.verification_uri, "https://github.com/login/device");
        assert_eq!(resp.expires_in, 900);
        assert_eq!(resp.interval, 5);
    }

    #[test]
    fn auth_error_debug_format() {
        let err = AuthError::Timeout;
        let debug = format!("{:?}", err);
        assert!(debug.contains("Timeout"));
    }

    #[test]
    fn auth_error_request_failed_display() {
        // Build a reqwest error by trying to parse an invalid URL
        let client = reqwest::Client::new();
        let err = client.get("://bad").build().unwrap_err();
        let auth_err = AuthError::RequestFailed(err);
        assert!(auth_err.to_string().contains("HTTP request failed"));
    }

    #[test]
    fn github_user_deserializes_all_fields() {
        let json = r#"{"login":"alice","id":42,"name":"Alice","email":"alice@example.com"}"#;
        let user: GitHubUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.login, "alice");
        assert_eq!(user.id, 42);
        assert_eq!(user.name, Some("Alice".to_string()));
        assert_eq!(user.email, Some("alice@example.com".to_string()));
    }

    #[test]
    fn github_user_deserializes_without_optional_fields() {
        let json = r#"{"login":"bob","id":7}"#;
        let user: GitHubUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.login, "bob");
        assert!(user.name.is_none());
        assert!(user.email.is_none());
    }

    #[tokio::test]
    async fn verify_token_returns_error_on_403() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&mock_server)
            .await;

        let auth = GitHubAuth::with_client_id("test-id".to_string())
            .with_api_base_url(mock_server.uri());

        let err = auth.verify_token("token").await.unwrap_err();
        match err {
            AuthError::DeviceFlowError {
                error,
                error_description,
            } => {
                assert_eq!(error, "verification_failed");
                assert_eq!(error_description, "forbidden");
            }
            other => panic!("Expected DeviceFlowError, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn verify_token_sends_user_agent_header() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/user"))
            .and(header("User-Agent", "nexor"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "login": "uauser",
                "id": 50,
                "name": null,
                "email": null
            })))
            .mount(&mock_server)
            .await;

        let auth = GitHubAuth::with_client_id("test-id".to_string())
            .with_api_base_url(mock_server.uri());

        let user = auth.verify_token("tok").await.unwrap();
        assert_eq!(user.login, "uauser");
    }

    #[test]
    fn configure_git_credentials_runs() {
        // This actually runs git config --global, so we just verify it doesn't panic
        // and returns Ok or Err (depending on environment).
        let result = GitHubAuth::configure_git_credentials("test-token-for-unit-test");
        // Clean up regardless
        let _ = GitHubAuth::remove_git_credentials();
        // In CI or local, git should be available
        assert!(result.is_ok());
    }

    #[test]
    fn token_response_success_fields() {
        let json = r#"{"access_token":"gho_xyz","token_type":"bearer","scope":"repo read:org"}"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        match resp {
            TokenResponse::Success {
                access_token,
                token_type,
                scope,
            } => {
                assert_eq!(access_token, "gho_xyz");
                assert_eq!(token_type, "bearer");
                assert_eq!(scope, "repo read:org");
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn token_response_error_with_empty_description() {
        let json = r#"{"error":"slow_down","error_description":""}"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        match resp {
            TokenResponse::Error {
                error,
                error_description,
            } => {
                assert_eq!(error, "slow_down");
                assert_eq!(error_description, Some("".to_string()));
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn device_code_response_debug() {
        let dc = DeviceCodeResponse {
            device_code: "dc".to_string(),
            user_code: "UC".to_string(),
            verification_uri: "https://example.com".to_string(),
            expires_in: 300,
            interval: 5,
        };
        let debug = format!("{:?}", dc);
        assert!(debug.contains("DeviceCodeResponse"));
        assert!(debug.contains("UC"));
    }

    #[test]
    fn github_user_debug() {
        let user = GitHubUser {
            login: "testuser".to_string(),
            id: 1,
            name: None,
            email: None,
        };
        let debug = format!("{:?}", user);
        assert!(debug.contains("testuser"));
    }
}
