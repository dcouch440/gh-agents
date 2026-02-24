//! Centralized environment configuration.
//!
//! All environment variables are read once at startup via `Env::load()` and
//! stored in an immutable struct. This avoids scattered `std::env::var()` calls
//! with inconsistent defaults and parsing.

use crate::constants;

/// Centralized environment configuration read once at startup.
#[derive(Debug, Clone)]
pub struct Env {
    // ── Database ─────────────────────────────────────────────────────────
    pub database_url: String,
    pub db_max_connections: u32,

    // ── Auth ─────────────────────────────────────────────────────────────
    pub jwt_secret: Option<String>,
    pub rust_env: String,

    // ── LLM Providers ────────────────────────────────────────────────────
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: String,
    pub xai_api_key: Option<String>,
    pub xai_model: String,
    pub ollama_enabled: bool,
    pub ollama_model: Option<String>,
    pub ollama_base_url: String,

    // ── Server ───────────────────────────────────────────────────────────
    pub cors_origins: Option<String>,
    pub static_dir: String,
    pub skip_rate_limit: bool,

    // ── GitHub ───────────────────────────────────────────────────────────
    pub github_token: Option<String>,

    // ── Debug ────────────────────────────────────────────────────────────
    pub debug_stream: bool,

    // ── VPN ──────────────────────────────────────────────────────────────
    pub wgeasy_api_url: Option<String>,
    pub wgeasy_password: Option<String>,
}

impl Env {
    /// Read all environment variables once. Call after `dotenvy::dotenv()`.
    pub fn load() -> Self {
        let jwt_secret_raw = std::env::var(constants::ENV_JWT_SECRET)
            .ok()
            .filter(|s| !s.is_empty());

        let rust_env =
            std::env::var(constants::ENV_RUST_ENV).unwrap_or_else(|_| "development".to_string());

        let database_url = std::env::var(constants::ENV_DATABASE_URL)
            .unwrap_or_else(|_| "postgres://nexor:nexor@localhost:5432/nexor".to_string());

        let db_max_connections = std::env::var(constants::ENV_DB_MAX_CONNECTIONS)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50);

        let anthropic_api_key = std::env::var(constants::ENV_ANTHROPIC_API_KEY)
            .ok()
            .filter(|s| !s.is_empty());

        let anthropic_model = std::env::var(constants::ENV_ANTHROPIC_MODEL)
            .unwrap_or_else(|_| constants::ANTHROPIC_DEFAULT_MODEL.to_string());

        let xai_api_key = std::env::var(constants::ENV_XAI_API_KEY)
            .ok()
            .filter(|s| !s.is_empty());

        let xai_model = std::env::var(constants::ENV_XAI_MODEL)
            .unwrap_or_else(|_| constants::XAI_DEFAULT_CHAT_MODEL.to_string());

        let ollama_enabled = std::env::var(constants::ENV_OLLAMA_ENABLED)
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let ollama_model = std::env::var(constants::ENV_OLLAMA_MODEL)
            .ok()
            .filter(|s| !s.is_empty());

        let ollama_base_url = std::env::var(constants::ENV_OLLAMA_BASE_URL)
            .unwrap_or_else(|_| constants::OLLAMA_DEFAULT_BASE_URL.to_string());

        let cors_origins = std::env::var(constants::ENV_CORS_ORIGINS)
            .ok()
            .filter(|s| !s.is_empty());

        let static_dir = std::env::var(constants::ENV_NEXOR_STATIC_DIR)
            .unwrap_or_else(|_| "ui/dist".to_string());

        let skip_rate_limit = std::env::var("NEXOR_SKIP_RATE_LIMIT")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let github_token = std::env::var(constants::ENV_GITHUB_TOKEN)
            .ok()
            .filter(|s| !s.is_empty());

        let debug_stream = std::env::var(constants::ENV_DEBUG_STREAM)
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let wgeasy_api_url = std::env::var("WGEASY_API_URL")
            .ok()
            .filter(|s| !s.is_empty());

        let wgeasy_password = std::env::var("WGEASY_PASSWORD").ok();

        Self {
            database_url,
            db_max_connections,
            jwt_secret: jwt_secret_raw,
            rust_env,
            anthropic_api_key,
            anthropic_model,
            xai_api_key,
            xai_model,
            ollama_enabled,
            ollama_model,
            ollama_base_url,
            cors_origins,
            static_dir,
            skip_rate_limit,
            github_token,
            debug_stream,
            wgeasy_api_url,
            wgeasy_password,
        }
    }

    /// Whether the current environment is production.
    pub fn is_production(&self) -> bool {
        self.rust_env.eq_ignore_ascii_case("production")
    }

    /// Build test defaults (no env vars read).
    #[cfg(test)]
    pub fn test_default() -> Self {
        Self {
            database_url: "postgres://test:test@localhost:5432/test".to_string(),
            db_max_connections: 5,
            jwt_secret: None,
            rust_env: "test".to_string(),
            anthropic_api_key: None,
            anthropic_model: constants::ANTHROPIC_DEFAULT_MODEL.to_string(),
            xai_api_key: None,
            xai_model: constants::XAI_DEFAULT_CHAT_MODEL.to_string(),
            ollama_enabled: false,
            ollama_model: None,
            ollama_base_url: constants::OLLAMA_DEFAULT_BASE_URL.to_string(),
            cors_origins: None,
            static_dir: "ui/dist".to_string(),
            skip_rate_limit: true,
            github_token: None,
            debug_stream: false,
            wgeasy_api_url: None,
            wgeasy_password: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_creates_valid_env() {
        let env = Env::test_default();
        assert_eq!(env.rust_env, "test");
        assert!(!env.is_production());
        assert!(!env.debug_stream);
        assert!(env.skip_rate_limit);
    }

    #[test]
    fn is_production_checks_rust_env() {
        let mut env = Env::test_default();
        assert!(!env.is_production());

        env.rust_env = "production".to_string();
        assert!(env.is_production());

        env.rust_env = "PRODUCTION".to_string();
        assert!(env.is_production());
    }
}
