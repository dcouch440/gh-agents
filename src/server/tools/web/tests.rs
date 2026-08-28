#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn is_web_tool_matches_every_declared_name() {
        for name in WEB_TOOLS {
            assert!(is_web_tool(name), "{name} should be a web tool");
        }
    }

    #[test]
    fn is_web_tool_rejects_non_web_tools() {
        for name in ["run_command", "read_file", "search_docs", "think", ""] {
            assert!(!is_web_tool(name), "{name} must not be a web tool");
        }
    }

    #[test]
    fn is_web_tool_is_case_sensitive() {
        // Tool names arrive verbatim from the model; a case-insensitive match
        // would let "Brave_Search" bypass the registry lookup that follows.
        assert!(!is_web_tool("Brave_Search"));
    }

    // resolve_tools silently drops a capability-assigned name that has no
    // registry definition, so the agent would simply have no tool and no
    // error anywhere. This is the only thing that catches it.
    #[test]
    fn web_tools_all_have_registry_definitions() {
        for name in WEB_TOOLS {
            let def = crate::tools::registry::get_tool_definition(name);
            assert!(def.is_some(), "{name} has no registry definition");
            assert_eq!(&def.unwrap().name, name);
        }
    }

    #[test]
    fn web_tool_schemas_are_well_formed() {
        for name in WEB_TOOLS {
            let t = crate::tools::registry::get_tool_definition(name).unwrap();
            assert!(!t.description.is_empty(), "{name} needs a description");
            assert_eq!(t.input_schema["type"], "object", "{name}");
            assert!(t.input_schema["properties"].is_object(), "{name}");
            assert!(t.input_schema["required"].is_array(), "{name}");
        }
    }

    // Web tools deliberately do NOT belong in execution_tools(), whose length
    // is asserted as 15 in src/server/tools/execution/tests.rs.
    #[test]
    fn web_tools_are_not_in_the_execution_tool_list() {
        let names: Vec<String> = crate::server::tools::execution::execution_tools()
            .into_iter()
            .map(|t| t.name)
            .collect();
        for name in WEB_TOOLS {
            assert!(
                !names.contains(&name.to_string()),
                "{name} must not be in execution_tools()"
            );
        }
    }

    #[test]
    fn web_tools_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for name in WEB_TOOLS {
            assert!(seen.insert(*name), "duplicate web tool: {name}");
        }
    }
}
