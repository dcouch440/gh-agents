//! Tests for credentials store

use super::*;
use tempfile::TempDir;

#[test]
fn save_and_load_credentials() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("credentials.json");

    let store = CredentialsStore::with_path(path);

    let creds = StoredCredentials {
        github_token: Some("test_token".to_string()),
        github_user: Some("testuser".to_string()),
        created_at: Some("2024-01-01".to_string()),
    };

    store.save(&creds).unwrap();

    let loaded = store.load().unwrap();
    assert_eq!(loaded.github_token, Some("test_token".to_string()));
    assert_eq!(loaded.github_user, Some("testuser".to_string()));
}

#[test]
fn missing_file_returns_not_found() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("nonexistent.json");

    let store = CredentialsStore::with_path(path);

    let result = store.load();
    assert!(matches!(result, Err(CredentialsError::NotFound)));
}

#[test]
fn clear_removes_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("credentials.json");

    let store = CredentialsStore::with_path(path.clone());

    store
        .save(&StoredCredentials {
            github_token: Some("token".to_string()),
            ..Default::default()
        })
        .unwrap();

    assert!(path.exists());

    store.clear().unwrap();

    assert!(!path.exists());
}

#[test]
fn is_authenticated_checks_token() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("credentials.json");

    let store = CredentialsStore::with_path(path);

    // Not authenticated initially
    assert!(!store.is_authenticated());

    // Save a token
    store
        .save(&StoredCredentials {
            github_token: Some("token".to_string()),
            ..Default::default()
        })
        .unwrap();

    // Now authenticated
    assert!(store.is_authenticated());
}

#[test]
fn default_credentials_are_empty() {
    let creds = StoredCredentials::default();
    assert!(creds.github_token.is_none());
    assert!(creds.github_user.is_none());
    assert!(creds.created_at.is_none());
}

#[cfg(unix)]
#[test]
fn credentials_have_restrictive_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("credentials.json");

    let store = CredentialsStore::with_path(path.clone());

    store
        .save(&StoredCredentials {
            github_token: Some("token".to_string()),
            ..Default::default()
        })
        .unwrap();

    let perms = std::fs::metadata(&path).unwrap().permissions();
    assert_eq!(perms.mode() & 0o777, 0o600);
}
