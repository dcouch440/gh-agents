//! Validation functions for the system node agent's repository files.
//!
//! Used by `complete_system` verification and (future) write-time validation.
//! All functions are pure — no IO except reading files from the base directory.

use std::collections::HashSet;
use std::path::Path;

use serde_json::Value;

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;

// ---------------------------------------------------------------------------
// Per-file validators
// ---------------------------------------------------------------------------

/// Validate `config.json` content. Returns Ok or an error message.
pub(crate) fn validate_config(content: &str) -> Result<(), String> {
    let val: Value =
        serde_json::from_str(content).map_err(|e| format!("invalid JSON: {e}"))?;

    let obj = val
        .as_object()
        .ok_or("expected a JSON object")?;

    require_non_empty_string(obj, "name")?;
    require_non_empty_string(obj, "description")?;
    Ok(())
}

/// Validate `topology.json` content. Returns Ok or an error message.
pub(crate) fn validate_topology(content: &str) -> Result<(), String> {
    let val: Value =
        serde_json::from_str(content).map_err(|e| format!("invalid JSON: {e}"))?;

    let obj = val.as_object().ok_or("expected a JSON object")?;

    let agents = obj
        .get("agents")
        .and_then(|v| v.as_object())
        .ok_or("missing or invalid \"agents\" object")?;

    for (slug, entry) in agents {
        let deps = entry
            .get("depends_on")
            .and_then(|v| v.as_array())
            .ok_or_else(|| format!("agent \"{slug}\" missing \"depends_on\" array"))?;

        for dep in deps {
            if !dep.is_string() {
                return Err(format!(
                    "agent \"{slug}\" has non-string value in depends_on"
                ));
            }
        }
    }

    Ok(())
}

/// Validate an `agents/{slug}.json` config. Returns Ok or an error message.
pub(crate) fn validate_agent(content: &str) -> Result<(), String> {
    let val: Value =
        serde_json::from_str(content).map_err(|e| format!("invalid JSON: {e}"))?;

    let obj = val.as_object().ok_or("expected a JSON object")?;

    require_non_empty_string(obj, "name")?;
    require_non_empty_string(obj, "system_prompt")?;
    require_non_empty_string(obj, "assignment")?;
    require_non_empty_string(obj, "expected_output")?;

    let caps = obj
        .get("capabilities")
        .and_then(|v| v.as_array())
        .ok_or("missing or invalid \"capabilities\" array")?;

    for cap in caps {
        if !cap.is_string() {
            return Err("capabilities must be an array of strings".to_string());
        }
    }

    Ok(())
}

fn require_non_empty_string(
    obj: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<(), String> {
    match obj.get(field).and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => Ok(()),
        Some(_) => Err(format!("\"{field}\" is empty")),
        None => Err(format!("missing required field \"{field}\"")),
    }
}

// ---------------------------------------------------------------------------
// Cross-reference validation
// ---------------------------------------------------------------------------

/// A single cross-reference error.
#[derive(Debug)]
pub(crate) struct CrossRefError {
    pub file: String,
    pub error: String,
}

/// Cross-reference topology slugs against agent files and config.json.
///
/// Returns an empty vec if everything matches.
pub(crate) fn cross_reference(base_dir: &Path) -> Vec<CrossRefError> {
    let mut errors = Vec::new();

    // Read topology.json
    let topology_path = base_dir.join("topology.json");
    let topology_content = match std::fs::read_to_string(&topology_path) {
        Ok(c) => c,
        Err(_) => {
            errors.push(CrossRefError {
                file: "topology.json".into(),
                error: "file does not exist".into(),
            });
            return errors;
        }
    };

    let topology_slugs: HashSet<String> = match serde_json::from_str::<Value>(&topology_content) {
        Ok(val) => val
            .get("agents")
            .and_then(|v| v.as_object())
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default(),
        Err(e) => {
            errors.push(CrossRefError {
                file: "topology.json".into(),
                error: format!("invalid JSON: {e}"),
            });
            return errors;
        }
    };

    // Check each topology slug has a matching agent file
    let agents_dir = base_dir.join("agents");
    for slug in &topology_slugs {
        let agent_path = agents_dir.join(format!("{slug}.json"));
        if !agent_path.exists() {
            errors.push(CrossRefError {
                file: format!("agents/{slug}.json"),
                error: "listed in topology.json but file does not exist".into(),
            });
        }
    }

    // Check for orphaned agent files not in topology
    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(slug) = name_str.strip_suffix(".json") {
                if !topology_slugs.contains(slug) {
                    errors.push(CrossRefError {
                        file: format!("agents/{slug}.json"),
                        error: "file exists but not listed in topology.json".into(),
                    });
                }
            }
        }
    }

    // Check config.json exists
    let config_path = base_dir.join("config.json");
    if !config_path.exists() {
        errors.push(CrossRefError {
            file: "config.json".into(),
            error: "file does not exist".into(),
        });
    }

    errors
}

// ---------------------------------------------------------------------------
// complete_system verify validation
// ---------------------------------------------------------------------------

/// Validate the `verify` claims from `complete_system` against the filesystem.
///
/// Returns `Ok(success_json)` if all claims hold, or `Err(error_json)` with
/// structured errors the agent can act on.
pub(crate) fn validate_verify(
    base_dir: &Path,
    verify: &Value,
) -> Result<Value, Value> {
    let mut errors: Vec<Value> = Vec::new();

    // topology_complete
    if verify["topology_complete"].as_bool() == Some(true) {
        let xref = cross_reference(base_dir);
        for err in xref {
            errors.push(serde_json::json!({
                "verify": "topology_complete",
                "file": err.file,
                "error": err.error,
            }));
        }
    }

    // agents_complete
    if verify["agents_complete"].as_bool() == Some(true) {
        let agents_dir = base_dir.join("agents");
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            if let Err(msg) = validate_agent(&content) {
                                errors.push(serde_json::json!({
                                    "verify": "agents_complete",
                                    "file": format!("agents/{name}"),
                                    "error": msg,
                                }));
                            }
                        }
                        Err(e) => {
                            errors.push(serde_json::json!({
                                "verify": "agents_complete",
                                "file": format!("agents/{name}"),
                                "error": format!("cannot read file: {e}"),
                            }));
                        }
                    }
                }
            }
        }
    }

    // config_accurate
    if verify["config_accurate"].as_bool() == Some(true) {
        let config_path = base_dir.join("config.json");
        match std::fs::read_to_string(&config_path) {
            Ok(content) => {
                if let Err(msg) = validate_config(&content) {
                    errors.push(serde_json::json!({
                        "verify": "config_accurate",
                        "file": "config.json",
                        "error": msg,
                    }));
                }
            }
            Err(_) => {
                errors.push(serde_json::json!({
                    "verify": "config_accurate",
                    "file": "config.json",
                    "error": "file does not exist",
                }));
            }
        }
    }

    if errors.is_empty() {
        Ok(serde_json::json!({ "status": "ok" }))
    } else {
        Err(serde_json::json!({
            "status": "verification_failed",
            "errors": errors,
        }))
    }
}
