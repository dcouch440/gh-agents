//! Variable resolution: `{variable}` interpolation and for-each element access.

use std::collections::HashMap;

use serde_json::Value as JsonValue;

/// Resolve `{variable}` references in a prompt template.
///
/// Supports dot-path access: `{features.content.0.name}`.
/// Scope: completed step outputs (from this workflow) + prior stage outputs.
pub fn resolve_variables(
    template: &str,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
) -> String {
    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Collect the variable path
            let mut path = String::new();
            let mut depth = 1;
            for inner in chars.by_ref() {
                if inner == '{' {
                    depth += 1;
                    path.push(inner);
                } else if inner == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    path.push(inner);
                } else {
                    path.push(inner);
                }
            }
            // Resolve the path
            let resolved = resolve_path(&path, outputs, prior_outputs);
            result.push_str(&resolved);
        } else {
            result.push(ch);
        }
    }

    result
}

/// Navigate a dot-path into the combined output map.
pub(super) fn resolve_path(
    path: &str,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
) -> String {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return format!("{{{}}}", path);
    }

    let var_name = parts[0];

    // Look in workflow outputs first, then prior stage outputs
    let root = outputs
        .get(var_name)
        .or_else(|| prior_outputs.get(var_name));

    match root {
        Some(value) => {
            let mut current = value.clone();
            for &part in &parts[1..] {
                current = if let Ok(idx) = part.parse::<usize>() {
                    current.get(idx).cloned().unwrap_or(JsonValue::Null)
                } else {
                    current.get(part).cloned().unwrap_or(JsonValue::Null)
                };
            }
            match &current {
                JsonValue::String(s) => s.clone(),
                JsonValue::Null => format!("{{{}}}", path),
                other => other.to_string(),
            }
        }
        None => format!("{{{}}}", path), // Unresolved, leave as-is
    }
}

/// For a for_each step, resolve the array to iterate over.
pub fn resolve_for_each_array(
    for_each_ref: &str,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
) -> Option<Vec<JsonValue>> {
    let parts: Vec<&str> = for_each_ref.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let var_name = parts[0];
    let root = outputs
        .get(var_name)
        .or_else(|| prior_outputs.get(var_name))?;

    let mut current = root.clone();
    for &part in &parts[1..] {
        current = if let Ok(idx) = part.parse::<usize>() {
            current.get(idx).cloned().unwrap_or(JsonValue::Null)
        } else {
            current.get(part).cloned().unwrap_or(JsonValue::Null)
        };
    }

    current.as_array().cloned()
}

/// Extract the for_each label from an element using the label field.
pub fn extract_for_each_label(element: &JsonValue, label_field: Option<&str>) -> Option<String> {
    let field = label_field?;
    element
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Resolve a for_each prompt where `$` represents the current element.
pub(super) fn resolve_for_each_prompt(
    template: &str,
    element: &JsonValue,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
) -> String {
    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut path = String::new();
            let mut depth = 1;
            for inner in chars.by_ref() {
                if inner == '{' {
                    depth += 1;
                    path.push(inner);
                } else if inner == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    path.push(inner);
                } else {
                    path.push(inner);
                }
            }

            // Check if path contains `$` for for_each element access
            if path.contains(".$") {
                let resolved = resolve_for_each_path(&path, element, outputs, prior_outputs);
                result.push_str(&resolved);
            } else {
                let resolved = resolve_path(&path, outputs, prior_outputs);
                result.push_str(&resolved);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Resolve a path containing `$` — e.g. `features.content.$.name`
/// The `$` is replaced with the current for_each element.
fn resolve_for_each_path(
    path: &str,
    element: &JsonValue,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
) -> String {
    let parts: Vec<&str> = path.split('.').collect();

    // Find the position of `$`
    let dollar_pos = parts.iter().position(|&p| p == "$");
    let Some(dollar_pos) = dollar_pos else {
        return resolve_path(path, outputs, prior_outputs);
    };

    // Navigate from element using parts after `$`
    let mut current = element.clone();
    for &part in &parts[dollar_pos + 1..] {
        current = if let Ok(idx) = part.parse::<usize>() {
            current.get(idx).cloned().unwrap_or(JsonValue::Null)
        } else {
            current.get(part).cloned().unwrap_or(JsonValue::Null)
        };
    }

    match &current {
        JsonValue::String(s) => s.clone(),
        JsonValue::Null => format!("{{{}}}", path),
        other => other.to_string(),
    }
}
