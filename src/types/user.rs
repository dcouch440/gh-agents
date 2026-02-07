//! User-related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a user
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A registered user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub github_id: Option<i64>,
    pub github_login: Option<String>,
    #[serde(skip_serializing)]
    pub github_token_encrypted: Option<String>,
    #[serde(default)]
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_id_new_creates_unique_ids() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn user_id_default_creates_new_id() {
        let id = UserId::default();
        assert_ne!(id.0, Uuid::nil());
    }

    #[test]
    fn user_id_display() {
        let uuid = Uuid::new_v4();
        let id = UserId(uuid);
        assert_eq!(format!("{}", id), format!("{}", uuid));
    }

    #[test]
    fn user_id_equality() {
        let uuid = Uuid::new_v4();
        let id1 = UserId(uuid);
        let id2 = UserId(uuid);
        assert_eq!(id1, id2);

        let id3 = UserId(Uuid::new_v4());
        assert_ne!(id1, id3);
    }

    #[test]
    fn user_id_serialization() {
        let id = UserId::new();
        let serialized = serde_json::to_string(&id).unwrap();
        let deserialized: UserId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn user_serialization_skips_sensitive_fields() {
        let user = User {
            id: UserId::new(),
            email: "test@example.com".to_string(),
            password_hash: Some("secret_hash".to_string()),
            github_id: Some(12345),
            github_login: Some("testuser".to_string()),
            github_token_encrypted: Some("encrypted_secret".to_string()),
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let serialized = serde_json::to_string(&user).unwrap();

        // Sensitive fields should not be present
        assert!(!serialized.contains("secret_hash"));
        assert!(!serialized.contains("encrypted_secret"));

        // Public fields should be present
        assert!(serialized.contains("test@example.com"));
        assert!(serialized.contains("testuser"));
    }

    #[test]
    fn user_deserialization() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "email": "user@example.com",
            "github_id": 99999,
            "github_login": "ghuser",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.email, "user@example.com");
        assert_eq!(user.github_id, Some(99999));
        assert_eq!(user.github_login, Some("ghuser".to_string()));
        assert_eq!(user.password_hash, None);
        assert_eq!(user.github_token_encrypted, None);
    }

    #[test]
    fn user_clone() {
        let user = User {
            id: UserId::new(),
            email: "test@example.com".to_string(),
            password_hash: Some("hash".to_string()),
            github_id: None,
            github_login: None,
            github_token_encrypted: None,
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let cloned = user.clone();
        assert_eq!(user.email, cloned.email);
        assert_eq!(user.password_hash, cloned.password_hash);
    }

    #[test]
    fn user_debug() {
        let user = User {
            id: UserId::new(),
            email: "debug@example.com".to_string(),
            password_hash: Some("hash".to_string()),
            github_id: Some(123),
            github_login: Some("debuguser".to_string()),
            github_token_encrypted: Some("token".to_string()),
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let debug = format!("{:?}", user);
        assert!(debug.contains("debug@example.com"));
        assert!(debug.contains("debuguser"));
    }

    #[test]
    fn user_id_hash() {
        use std::collections::HashSet;

        let id1 = UserId::new();
        let id2 = UserId::new();

        let mut set = HashSet::new();
        set.insert(id1);
        set.insert(id2);

        assert_eq!(set.len(), 2);
        assert!(set.contains(&id1));
        assert!(set.contains(&id2));
    }

    #[test]
    fn user_with_github_account() {
        let user = User {
            id: UserId::new(),
            email: "github@example.com".to_string(),
            password_hash: None,
            github_id: Some(54321),
            github_login: Some("octocat".to_string()),
            github_token_encrypted: Some("encrypted".to_string()),
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(user.password_hash.is_none());
        assert!(user.github_id.is_some());
        assert_eq!(user.github_login.unwrap(), "octocat");
    }

    #[test]
    fn user_with_password_only() {
        let user = User {
            id: UserId::new(),
            email: "password@example.com".to_string(),
            password_hash: Some("bcrypt_hash".to_string()),
            github_id: None,
            github_login: None,
            github_token_encrypted: None,
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(user.password_hash.is_some());
        assert!(user.github_id.is_none());
    }
}
