//! Validation logic for config files

use super::types::*;
use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Validate all config files before syncing
pub fn validate_all(
    capabilities: &CapabilitiesYaml,
    tool_assignments: &ToolAssignmentsYaml,
) -> Result<()> {
    validate_capabilities(capabilities)?;
    validate_tool_assignments(tool_assignments, capabilities)?;
    Ok(())
}

/// Validate capability definitions
fn validate_capabilities(caps: &CapabilitiesYaml) -> Result<()> {
    let key_regex = Regex::new(r"^[a-z][a-z0-9_]*$").unwrap();
    let mut seen_keys = HashSet::new();

    for cap in &caps.capabilities {
        // Validate key format
        if !key_regex.is_match(&cap.key) {
            bail!(
                "Invalid capability key '{}': must match ^[a-z][a-z0-9_]*$",
                cap.key
            );
        }

        // Check for duplicates
        if !seen_keys.insert(&cap.key) {
            bail!("Duplicate capability key '{}'", cap.key);
        }

        // Validate safety level
        validate_safety_level(&cap.safety_level)
            .with_context(|| format!("capability '{}'", cap.key))?;

        // Validate category
        if cap.category.is_empty() {
            bail!("Capability '{}' has empty category", cap.key);
        }
    }

    Ok(())
}

/// Validate tool assignments
fn validate_tool_assignments(
    assignments: &ToolAssignmentsYaml,
    capabilities: &CapabilitiesYaml,
) -> Result<()> {
    // Build set of valid capability keys
    let valid_caps: HashSet<_> = capabilities
        .capabilities
        .iter()
        .map(|c| c.key.as_str())
        .collect();

    for (tool_name, assignment) in &assignments.tool_assignments {
        // Validate tool name
        if tool_name.is_empty() {
            bail!("Tool assignment has empty name");
        }

        // Validate capabilities exist
        for cap_key in &assignment.capabilities {
            if !valid_caps.contains(cap_key.as_str()) {
                bail!(
                    "Tool '{}' references unknown capability '{}'",
                    tool_name,
                    cap_key
                );
            }
        }

        // Validate at least one capability
        if assignment.capabilities.is_empty() {
            bail!("Tool '{}' has no capabilities assigned", tool_name);
        }
    }

    Ok(())
}

/// Validate constraint configuration
pub fn validate_constraint(constraint: &ConstraintConfig) -> Result<()> {
    // Validate config_type matches value type
    match constraint.config_type.as_str() {
        "integer" => {
            if !constraint.value.is_i64() {
                bail!(
                    "Constraint value must be integer, got: {}",
                    constraint.value
                );
            }
        }
        "float" => {
            if !constraint.value.is_f64() && !constraint.value.is_i64() {
                bail!("Constraint value must be float, got: {}", constraint.value);
            }
        }
        "boolean" => {
            if !constraint.value.is_boolean() {
                bail!(
                    "Constraint value must be boolean, got: {}",
                    constraint.value
                );
            }
        }
        "string" => {
            if !constraint.value.is_string() {
                bail!("Constraint value must be string, got: {}", constraint.value);
            }
        }
        _ => {
            bail!("Invalid config_type: '{}'", constraint.config_type);
        }
    }

    Ok(())
}

/// Validate routing strategy configuration
pub fn validate_routing_strategy(strategy: &RoutingStrategyYaml) -> Result<()> {
    // Validate subtask dependencies form a DAG (no cycles)
    validate_dag(&strategy.subtasks)?;

    // Validate aggregation mode
    match strategy.aggregation_mode.as_str() {
        "final_output" | "all_outputs" | "merge" => {}
        _ => bail!(
            "Invalid aggregation_mode '{}': must be 'final_output', 'all_outputs', or 'merge'",
            strategy.aggregation_mode
        ),
    }

    // Validate subtask IDs are unique and build set for dependency checking
    let mut seen_ids = HashSet::new();
    for subtask in &strategy.subtasks {
        if !seen_ids.insert(subtask.id.as_str()) {
            bail!("Duplicate subtask id '{}'", subtask.id);
        }
    }

    // Validate dependencies reference existing subtasks
    for subtask in &strategy.subtasks {
        for dep in &subtask.depends_on {
            if !seen_ids.contains(dep.as_str()) {
                bail!(
                    "Subtask '{}' depends on unknown subtask '{}'",
                    subtask.id,
                    dep
                );
            }
        }
    }

    Ok(())
}

/// Validate subtask dependencies form a DAG (no cycles) using Kahn's algorithm
fn validate_dag(subtasks: &[Subtask]) -> Result<()> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

    // Build adjacency list and in-degree count
    for subtask in subtasks {
        in_degree.entry(&subtask.id).or_insert(0);
        graph.entry(&subtask.id).or_insert_with(Vec::new);

        for dep in &subtask.depends_on {
            *in_degree.entry(&subtask.id).or_insert(0) += 1;
            graph
                .entry(dep.as_str())
                .or_insert_with(Vec::new)
                .push(&subtask.id);
        }
    }

    // Kahn's algorithm for topological sort
    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut visited = 0;

    while let Some(node) = queue.pop() {
        visited += 1;
        if let Some(neighbors) = graph.get(node) {
            for &neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor);
                    }
                }
            }
        }
    }

    if visited != subtasks.len() {
        bail!("Routing strategy has circular dependencies (cycle detected in subtask DAG)");
    }

    Ok(())
}

/// Validate safety level enum
fn validate_safety_level(level: &str) -> Result<()> {
    match level {
        "safe" | "caution" | "unsafe" => Ok(()),
        _ => Err(anyhow!(
            "Invalid safety_level '{}': must be 'safe', 'caution', or 'unsafe'",
            level
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_key_validation() {
        let valid = CapabilitiesYaml {
            capabilities: vec![Capability {
                key: "file_read".to_string(),
                display_name: "File Reading".to_string(),
                category: "filesystem".to_string(),
                safety_level: "safe".to_string(),
                description: "Read files".to_string(),
                examples: vec![],
                notes: None,
                requires_approval: false,
                default_enabled: true,
            }],
        };
        assert!(validate_capabilities(&valid).is_ok());

        // Invalid: starts with uppercase
        let invalid = CapabilitiesYaml {
            capabilities: vec![Capability {
                key: "File_Read".to_string(),
                display_name: "File Reading".to_string(),
                category: "filesystem".to_string(),
                safety_level: "safe".to_string(),
                description: "Read files".to_string(),
                examples: vec![],
                notes: None,
                requires_approval: false,
                default_enabled: true,
            }],
        };
        assert!(validate_capabilities(&invalid).is_err());
    }

    #[test]
    fn test_safety_level_validation() {
        assert!(validate_safety_level("safe").is_ok());
        assert!(validate_safety_level("caution").is_ok());
        assert!(validate_safety_level("unsafe").is_ok());
        assert!(validate_safety_level("invalid").is_err());
    }

    #[test]
    fn test_dag_validation() {
        // Valid DAG
        let valid = vec![
            Subtask {
                id: "a".to_string(),
                task_name: "A".to_string(),
                agent_role: "role_a".to_string(),
                agent_id: "uuid-a".to_string(),
                tools: vec![],
                prompt_template: "".to_string(),
                depends_on: vec![],
                input_mapping: HashMap::new(),
                output_schema: None,
            },
            Subtask {
                id: "b".to_string(),
                task_name: "B".to_string(),
                agent_role: "role_b".to_string(),
                agent_id: "uuid-b".to_string(),
                tools: vec![],
                prompt_template: "".to_string(),
                depends_on: vec!["a".to_string()],
                input_mapping: HashMap::new(),
                output_schema: None,
            },
        ];
        assert!(validate_dag(&valid).is_ok());

        // Circular dependency
        let circular = vec![
            Subtask {
                id: "a".to_string(),
                task_name: "A".to_string(),
                agent_role: "role_a".to_string(),
                agent_id: "uuid-a".to_string(),
                tools: vec![],
                prompt_template: "".to_string(),
                depends_on: vec!["b".to_string()],
                input_mapping: HashMap::new(),
                output_schema: None,
            },
            Subtask {
                id: "b".to_string(),
                task_name: "B".to_string(),
                agent_role: "role_b".to_string(),
                agent_id: "uuid-b".to_string(),
                tools: vec![],
                prompt_template: "".to_string(),
                depends_on: vec!["a".to_string()],
                input_mapping: HashMap::new(),
                output_schema: None,
            },
        ];
        assert!(validate_dag(&circular).is_err());
    }
}
