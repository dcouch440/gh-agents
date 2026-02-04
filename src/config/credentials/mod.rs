//! Secure credential storage for GitHub tokens

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CredentialsError {
    #[error("credentials file not found")]
    NotFound,

    #[error("failed to read credentials: {0}")]
    ReadError(String),

    #[error("failed to write credentials: {0}")]
    WriteError(String),

    #[error("failed to parse credentials: {0}")]
    ParseError(String),

    #[error("credentials directory not found and could not be created")]
    DirectoryError,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoredCredentials {
    pub github_token: Option<String>,
    #[serde(default)]
    pub github_user: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

pub struct CredentialsStore {
    path: PathBuf,
}

impl CredentialsStore {
    pub fn new() -> Self {
        let path = Self::default_path();
        Self { path }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nexor")
            .join("credentials.json")
    }

    /// Save credentials to disk
    pub fn save(&self, credentials: &StoredCredentials) -> Result<(), CredentialsError> {
        // Ensure directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|_| CredentialsError::DirectoryError)?;
        }

        // Serialize
        let json = serde_json::to_string_pretty(credentials)
            .map_err(|e| CredentialsError::WriteError(e.to_string()))?;

        // Write file
        fs::write(&self.path, &json).map_err(|e| CredentialsError::WriteError(e.to_string()))?;

        // Set restrictive permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.path)
                .map_err(|e| CredentialsError::WriteError(e.to_string()))?
                .permissions();
            perms.set_mode(0o600); // Owner read/write only
            fs::set_permissions(&self.path, perms)
                .map_err(|e| CredentialsError::WriteError(e.to_string()))?;
        }

        tracing::debug!(path = %self.path.display(), "Credentials saved");
        Ok(())
    }

    /// Load credentials from disk
    pub fn load(&self) -> Result<StoredCredentials, CredentialsError> {
        if !self.path.exists() {
            return Err(CredentialsError::NotFound);
        }

        let json = fs::read_to_string(&self.path)
            .map_err(|e| CredentialsError::ReadError(e.to_string()))?;

        let credentials: StoredCredentials =
            serde_json::from_str(&json).map_err(|e| CredentialsError::ParseError(e.to_string()))?;

        Ok(credentials)
    }

    /// Get GitHub token, checking env var first, then stored credentials
    pub fn get_github_token(&self) -> Option<String> {
        // Check environment variable first (allows override)
        if let Ok(token) = std::env::var(crate::constants::ENV_GITHUB_TOKEN) {
            if !token.is_empty() {
                return Some(token);
            }
        }

        // Fall back to stored credentials
        self.load().ok().and_then(|c| c.github_token)
    }

    /// Check if we have valid GitHub credentials
    pub fn is_authenticated(&self) -> bool {
        self.get_github_token().is_some()
    }

    /// Clear stored credentials
    pub fn clear(&self) -> Result<(), CredentialsError> {
        if self.path.exists() {
            fs::remove_file(&self.path).map_err(|e| CredentialsError::WriteError(e.to_string()))?;
        }
        Ok(())
    }

    /// Get the path to the credentials file
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Default for CredentialsStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
