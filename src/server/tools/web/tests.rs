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

    #[test]
    fn web_tools_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for name in WEB_TOOLS {
            assert!(seen.insert(*name), "duplicate web tool: {name}");
        }
    }
}
