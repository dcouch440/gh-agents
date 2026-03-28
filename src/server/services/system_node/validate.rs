//! Validation functions for the system node agent's repository files.
//!
//! Used by `complete_system` verification and (future) write-time validation.
//! All functions are pure — no IO except reading files from the base directory.

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

/// Matches save/write verb followed by a filename (case-insensitive).
static PRESCRIBED_FILENAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(save|write|output|export|store)\s+(it\s+)?(as|to|in|into)\s+[\w.-]+\.(md|json|txt|csv|html|py|js|yaml|yml|xml)\b",
    )
    .unwrap()
});

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;

// ---------------------------------------------------------------------------
// Per-file validators
// ---------------------------------------------------------------------------

/// Validate `config.json` content. Returns Ok or an error message.
pub(crate) fn validate_config(content: &str) -> Result<(), String> {
    let val: Value = serde_json::from_str(content).map_err(|e| format!("invalid JSON: {e}"))?;

    let obj = val.as_object().ok_or("expected a JSON object")?;

    require_non_empty_string(obj, "name")?;
    require_non_empty_string(obj, "description")?;
    Ok(())
}

/// Validate `topology.json` content. Returns Ok or an error message.
pub(crate) fn validate_topology(content: &str) -> Result<(), String> {
    let val: Value = serde_json::from_str(content).map_err(|e| format!("invalid JSON: {e}"))?;

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
    let val: Value = serde_json::from_str(content).map_err(|e| format!("invalid JSON: {e}"))?;

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
// Quality validators
// ---------------------------------------------------------------------------

/// Check assignment/expected_output for prescribed filenames.
///
/// Returns `(field, message)` pairs for each violation.
pub(crate) fn check_prescribed_filenames(content: &str) -> Vec<(String, String)> {
    let mut issues = Vec::new();
    let val: Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return issues,
    };
    let obj = match val.as_object() {
        Some(o) => o,
        None => return issues,
    };

    for field in ["assignment", "expected_output"] {
        if let Some(text) = obj.get(field).and_then(|v| v.as_str()) {
            for m in PRESCRIBED_FILENAME_RE.find_iter(text) {
                issues.push((
                    field.into(),
                    format!(
                        "{field} prescribes filename '{}' — let the agent decide what to produce and where",
                        m.as_str()
                    ),
                ));
            }
        }
    }
    issues
}

/// Check system_prompt word count against minimum threshold.
pub(crate) fn check_prompt_length(content: &str, min_words: usize) -> Option<String> {
    let val: Value = serde_json::from_str(content).ok()?;
    let sp = val.get("system_prompt")?.as_str()?;
    let word_count = sp.split_whitespace().count();
    if word_count < min_words {
        Some(format!(
            "system_prompt is only {word_count} words (minimum {min_words}) \
             — add behavioral instructions (methodology, criteria, boundaries)"
        ))
    } else {
        None
    }
}

/// Check assignment word count against user input word count.
///
/// Assignment should expand on user intent, not compress it.
pub(crate) fn check_assignment_expansion(content: &str, user_text_words: usize) -> Option<String> {
    let val: Value = serde_json::from_str(content).ok()?;
    let assignment = val.get("assignment")?.as_str()?;
    let assignment_words = assignment.split_whitespace().count();
    if assignment_words < user_text_words {
        Some(format!(
            "assignment is {assignment_words} words but user input was \
             {user_text_words} words — assignment should expand on user \
             intent, not compress it"
        ))
    } else {
        None
    }
}

/// Extract word count from `<user_text>` block in the instruction.
///
/// Returns `None` if no `<user_text>` block is found (e.g., update/propagation).
pub(crate) fn extract_user_text_words(instruction: &str) -> Option<usize> {
    let start = instruction.find("<user_text>")? + "<user_text>".len();
    let end = instruction.find("</user_text>")?;
    if start >= end {
        return None;
    }
    Some(instruction[start..end].split_whitespace().count())
}

/// Count the number of agents in the topology file.
fn count_topology_agents(base_dir: &Path) -> usize {
    let topology_path = base_dir.join("topology.json");
    std::fs::read_to_string(&topology_path)
        .ok()
        .and_then(|c| serde_json::from_str::<Value>(&c).ok())
        .and_then(|v| v.get("agents")?.as_object().map(|o| o.len()))
        .unwrap_or(0)
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
///
/// `user_text_words` is the word count from the `<user_text>` instruction block,
/// used by `assignments_expanded`. Pass `None` for update/propagation scenarios.
pub(crate) fn validate_verify(
    base_dir: &Path,
    verify: &Value,
    user_text_words: Option<usize>,
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

    // no_filenames_prescribed
    if verify["no_filenames_prescribed"].as_bool() == Some(true) {
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
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        for (field, msg) in check_prescribed_filenames(&content) {
                            errors.push(serde_json::json!({
                                "verify": "no_filenames_prescribed",
                                "file": format!("agents/{name}"),
                                "field": field,
                                "error": msg,
                            }));
                        }
                    }
                }
            }
        }
    }

    // prompts_not_trivial
    if verify["prompts_not_trivial"].as_bool() == Some(true) {
        let agent_count = count_topology_agents(base_dir);
        let min_words = if agent_count > 1 { 20 } else { 10 };
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
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Some(msg) = check_prompt_length(&content, min_words) {
                            errors.push(serde_json::json!({
                                "verify": "prompts_not_trivial",
                                "file": format!("agents/{name}"),
                                "error": msg,
                            }));
                        }
                    }
                }
            }
        }
    }

    // assignments_expanded
    if verify["assignments_expanded"].as_bool() == Some(true) {
        if let Some(user_words) = user_text_words {
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
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Some(msg) = check_assignment_expansion(&content, user_words) {
                                errors.push(serde_json::json!({
                                    "verify": "assignments_expanded",
                                    "file": format!("agents/{name}"),
                                    "error": msg,
                                }));
                            }
                        }
                    }
                }
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
