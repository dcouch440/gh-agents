//! Prompt registry — loads, caches, and renders markdown prompt templates.
//!
//! At startup, loads all `.md` files from the `prompts/` directory tree.
//! Keys are path-based: `"system/distiller"`, `"agents/worker"`, etc.
//! Templates use `{variable}` syntax for substitution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::error::HubError;

/// Registry of prompt templates loaded from disk.
///
/// Stored in `AppState` and shared across all execution strategies.
/// Prompts are immutable after loading — restart to pick up changes.
#[derive(Debug, Clone)]
pub struct PromptRegistry {
    prompts: HashMap<String, String>,
    base_dir: PathBuf,
}

impl PromptRegistry {
    /// Load all `.md` files from the given directory tree.
    ///
    /// Keys are derived from relative paths with the `.md` extension stripped:
    /// `prompts/system/distiller.md` → `"system/distiller"`
    pub fn load_from_dir(base: &Path) -> Result<Self, HubError> {
        let mut prompts = HashMap::new();

        if !base.exists() {
            tracing::warn!("prompts directory does not exist: {}", base.display());
            return Ok(Self {
                prompts,
                base_dir: base.to_path_buf(),
            });
        }

        load_recursive(base, base, &mut prompts)?;

        tracing::info!("loaded {} prompts from {}", prompts.len(), base.display());

        Ok(Self {
            prompts,
            base_dir: base.to_path_buf(),
        })
    }

    /// Create an empty registry (for tests or when prompts dir is missing).
    pub fn empty() -> Self {
        Self {
            prompts: HashMap::new(),
            base_dir: PathBuf::new(),
        }
    }

    /// Get a raw prompt by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.prompts.get(key).map(|s| s.as_str())
    }

    /// Get a prompt or return an error.
    pub fn require(&self, key: &str) -> Result<&str, HubError> {
        self.get(key).ok_or_else(|| HubError::PromptNotFound { key: key.to_string() })
    }

    /// Render a prompt with `{variable}` substitution.
    ///
    /// Variables in the template like `{messages}` or `{schema}` are replaced
    /// with values from the provided map. Unknown variables are left as-is.
    pub fn render(&self, key: &str, vars: &HashMap<String, String>) -> Result<String, HubError> {
        let template = self.require(key)?;
        Ok(render_template(template, vars))
    }

    /// Render an arbitrary template string (not from registry) with variable substitution.
    pub fn render_inline(template: &str, vars: &HashMap<String, String>) -> String {
        render_template(template, vars)
    }

    /// List all loaded prompt keys.
    pub fn keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.prompts.keys().map(|s| s.as_str()).collect();
        keys.sort();
        keys
    }

    /// Number of loaded prompts.
    pub fn len(&self) -> usize {
        self.prompts.len()
    }

    /// Whether the registry has no prompts.
    pub fn is_empty(&self) -> bool {
        self.prompts.is_empty()
    }

    /// The directory prompts were loaded from.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

/// Recursively load `.md` files from a directory.
fn load_recursive(base: &Path, current: &Path, prompts: &mut HashMap<String, String>) -> Result<(), HubError> {
    let entries = std::fs::read_dir(current).map_err(|e| anyhow::anyhow!("failed to read directory {}: {}", current.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| anyhow::anyhow!("failed to read dir entry in {}: {}", current.display(), e))?;
        let path = entry.path();

        if path.is_dir() {
            load_recursive(base, &path, prompts)?;
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            let key = path.strip_prefix(base).unwrap_or(&path).with_extension("").to_string_lossy().replace('\\', "/"); // normalize Windows paths

            let content = std::fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("failed to read prompt {}: {}", path.display(), e))?;

            tracing::debug!("loaded prompt: {}", key);
            prompts.insert(key, content);
        }
    }

    Ok(())
}

/// Replace `{variable}` patterns in a template string.
fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let pattern = format!("{{{}}}", key);
        result = result.replace(&pattern, value);
    }
    result
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
