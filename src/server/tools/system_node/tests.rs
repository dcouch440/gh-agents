#[cfg(test)]
mod tests {
    use crate::server::tools::system_node::complete_system_tool;

    #[test]
    fn complete_system_tool_schema() {
        let tool = complete_system_tool();
        assert_eq!(tool.name, "complete_system");

        let required = tool.input_schema["required"].as_array().unwrap();
        let names: Vec<&str> = required
            .iter()
            .filter_map(|v: &serde_json::Value| v.as_str())
            .collect();
        assert!(names.contains(&"summary"));
        assert!(names.contains(&"verify"));

        let verify_props = &tool.input_schema["properties"]["verify"]["properties"];
        assert!(verify_props["topology_complete"].is_object());
        assert!(verify_props["agents_complete"].is_object());
        assert!(verify_props["config_accurate"].is_object());
    }
}
