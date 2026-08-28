#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::CapabilityRegistry;

    fn config_dir() -> &'static Path {
        Path::new("config")
    }

    #[test]
    fn load_from_yaml() {
        let registry = CapabilityRegistry::load(config_dir()).unwrap();
        // Should have built reverse index from tool_assignments.yaml
        assert!(!registry.capability_to_tools.is_empty());
    }

    #[test]
    fn resolve_single_capability() {
        let registry = CapabilityRegistry::load(config_dir()).unwrap();
        let (tools, names) = registry.resolve_tools(&["file_read".to_string()]);

        assert!(!tools.is_empty());
        assert!(names.contains(&"read_file".to_string()));
    }

    #[test]
    fn resolve_multiple_capabilities() {
        let registry = CapabilityRegistry::load(config_dir()).unwrap();
        let (tools, names) =
            registry.resolve_tools(&["file_read".to_string(), "file_write".to_string()]);

        assert!(tools.len() >= 2);
        assert!(names.contains(&"read_file".to_string()));
        // file_write is provided by both write_file and edit_file
        assert!(
            names.contains(&"write_file".to_string()) || names.contains(&"edit_file".to_string())
        );
    }

    #[test]
    fn resolve_deduplicates_tools() {
        let registry = CapabilityRegistry::load(config_dir()).unwrap();
        // file_read and file_metadata are both provided by read_file
        let (tools, names) =
            registry.resolve_tools(&["file_read".to_string(), "file_metadata".to_string()]);

        // read_file should appear only once despite providing both capabilities
        let read_file_count = names.iter().filter(|n| *n == "read_file").count();
        assert_eq!(read_file_count, 1);
        assert!(!tools.is_empty());
    }

    #[test]
    fn resolve_unknown_capability_returns_empty() {
        let registry = CapabilityRegistry::load(config_dir()).unwrap();
        let (tools, names) = registry.resolve_tools(&["nonexistent_capability".to_string()]);

        assert!(tools.is_empty());
        assert!(names.is_empty());
    }

    #[test]
    fn resolve_empty_input_returns_empty() {
        let registry = CapabilityRegistry::load(config_dir()).unwrap();
        let (tools, names) = registry.resolve_tools(&[]);

        assert!(tools.is_empty());
        assert!(names.is_empty());
    }

    #[test]
    fn empty_registry() {
        let registry = CapabilityRegistry::empty();
        let (tools, names) = registry.resolve_tools(&["file_read".to_string()]);

        assert!(tools.is_empty());
        assert!(names.is_empty());
    }
}

#[cfg(test)]
mod web_capability_tests {
    use super::super::*;
    use std::path::Path;

    fn registry() -> CapabilityRegistry {
        CapabilityRegistry::load(Path::new("config")).expect("shipped config should load")
    }

    #[test]
    fn web_search_resolves_to_brave_search() {
        let (tools, names) = registry().resolve_tools(&["web_search".to_string()]);
        assert_eq!(names, vec!["brave_search"], "{names:?}");
        assert_eq!(tools.len(), 1);
    }

    #[test]
    fn web_fetch_resolves_to_read_webpage() {
        let (_, names) = registry().resolve_tools(&["web_fetch".to_string()]);
        assert_eq!(names, vec!["read_webpage"], "{names:?}");
    }

    #[test]
    fn a_research_agent_gets_exactly_the_two_web_tools() {
        let (_, mut names) =
            registry().resolve_tools(&["web_search".to_string(), "web_fetch".to_string()]);
        names.sort();
        assert_eq!(names, vec!["brave_search", "read_webpage"]);
    }

    // resolve_tools silently drops a name with no registry definition, so a
    // typo in the YAML would leave an agent toolless with nothing logged.
    #[test]
    fn every_assigned_tool_has_a_static_definition() {
        let yaml = std::fs::read_to_string("config/system/tool_assignments.yaml").unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let assignments = parsed["tool_assignments"].as_mapping().unwrap();
        for (name, _) in assignments {
            let name = name.as_str().unwrap();
            assert!(
                crate::tools::registry::get_tool_definition(name).is_some(),
                "{name} is assigned a capability but has no registry definition"
            );
        }
    }

    // Every capability a tool claims must exist in the taxonomy, or the
    // reverse index points at a key nothing can ever request.
    #[test]
    fn every_claimed_capability_exists_in_the_taxonomy() {
        let caps: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string("config/system/capabilities.yaml").unwrap(),
        )
        .unwrap();
        let known: Vec<String> = caps["capabilities"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|c| c["key"].as_str().unwrap().to_string())
            .collect();

        let assign: serde_yaml::Value = serde_yaml::from_str(
            &std::fs::read_to_string("config/system/tool_assignments.yaml").unwrap(),
        )
        .unwrap();
        for (tool, body) in assign["tool_assignments"].as_mapping().unwrap() {
            for cap in body["capabilities"].as_sequence().unwrap() {
                let cap = cap.as_str().unwrap();
                assert!(
                    known.contains(&cap.to_string()),
                    "{} claims unknown capability {cap}",
                    tool.as_str().unwrap()
                );
            }
        }
    }
}
