#[cfg(test)]
mod tests {
    use crate::server::hub::strategies::chat::tools::resolve_step_tools;

    #[test]
    fn dispatch_tools_match_workforce_tools() {
        let tools = resolve_step_tools("workforce");
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        // Must include universal tools
        assert!(tool_names.contains(&"set_node_name"));
        assert!(tool_names.contains(&"set_node_description"));
        assert!(tool_names.contains(&"think"));
        assert!(tool_names.contains(&"update_notes"));

        // Must include workforce tools
        assert!(tool_names.contains(&"set_task"));
        assert!(tool_names.contains(&"add_agent"));
        assert!(tool_names.contains(&"update_agent"));
        assert!(tool_names.contains(&"remove_agent"));
        assert!(tool_names.contains(&"add_deliverable"));
        assert!(tool_names.contains(&"update_deliverable"));
        assert!(tool_names.contains(&"remove_deliverable"));
        assert!(tool_names.contains(&"set_dependency"));
        assert!(tool_names.contains(&"remove_dependency"));
        assert!(tool_names.contains(&"set_capabilities"));
        assert!(tool_names.contains(&"set_failure_mode"));
    }

    #[test]
    fn dispatch_tools_include_render_panel() {
        let tools = resolve_step_tools("workforce");
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"render_panel"));
    }
}
