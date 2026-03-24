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
