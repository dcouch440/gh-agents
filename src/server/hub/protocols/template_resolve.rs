//! Template variable resolution for protocol prompt templates.
//!
//! Uses `{{.Namespace.field}}` syntax (dot-prefixed double braces) to distinguish
//! from the DAG-level `{variable}` interpolation in `prompt_registry` and `dag/utils`.
//!
//! Protocol templates use the `Protocol` namespace:
//! - `{{.Protocol.available_agents}}` — formatted port listing
//! - `{{.Protocol.decisions}}` — decision option list
//! - `{{.Protocol.schema_context}}` — optional schema description

use std::collections::HashMap;

/// Resolve `{{.Namespace.field}}` placeholders in a template string.
///
/// Keys in `vars` should be the full dotted path without the leading dot
/// (e.g., `"Protocol.available_agents"`). The function matches and replaces
/// `{{.Protocol.available_agents}}` in the template.
///
/// Unknown variables are left as-is — no error, no removal.
pub fn resolve_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let pattern = format!("{{{{.{}}}}}", key);
        result = result.replace(&pattern, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_single_variable() {
        let mut vars = HashMap::new();
        vars.insert(
            "Protocol.available_agents".to_string(),
            "Agent A\nAgent B".to_string(),
        );
        let result = resolve_template("Agents:\n{{.Protocol.available_agents}}", &vars);
        assert_eq!(result, "Agents:\nAgent A\nAgent B");
    }

    #[test]
    fn resolves_multiple_variables() {
        let mut vars = HashMap::new();
        vars.insert(
            "Protocol.decisions".to_string(),
            "approve, reject".to_string(),
        );
        vars.insert(
            "Protocol.available_agents".to_string(),
            "Agent X".to_string(),
        );
        let result = resolve_template(
            "Decide: {{.Protocol.decisions}}\n{{.Protocol.available_agents}}",
            &vars,
        );
        assert_eq!(result, "Decide: approve, reject\nAgent X");
    }

    #[test]
    fn leaves_unknown_variables_unchanged() {
        let vars = HashMap::new();
        let template = "Hello {{.Unknown.field}}";
        let result = resolve_template(template, &vars);
        assert_eq!(result, template);
    }

    #[test]
    fn empty_vars_returns_template_unchanged() {
        let vars = HashMap::new();
        let template = "No variables here";
        let result = resolve_template(template, &vars);
        assert_eq!(result, template);
    }

    #[test]
    fn single_braces_not_replaced() {
        let mut vars = HashMap::new();
        vars.insert("Protocol.field".to_string(), "value".to_string());
        let result = resolve_template("{dag_variable} and {{.Protocol.field}}", &vars);
        assert_eq!(result, "{dag_variable} and value");
    }

    #[test]
    fn dag_style_variable_not_affected() {
        let mut vars = HashMap::new();
        vars.insert(
            "Protocol.available_agents".to_string(),
            "Agents".to_string(),
        );
        let result = resolve_template("{anchor_output} then {{.Protocol.available_agents}}", &vars);
        assert_eq!(result, "{anchor_output} then Agents");
    }

    #[test]
    fn empty_template_returns_empty() {
        let vars = HashMap::new();
        assert_eq!(resolve_template("", &vars), "");
    }
}
