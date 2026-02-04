#[cfg(test)]
mod tests {
    use super::*;

    // Test: resolve with no router
    #[tokio::test]
    async fn test_resolve_no_router() {
        // Setup mock repo with agent (router_id = None)
        // Call resolve()
        // Assert: selected_mode_id is None
        // Assert: agent's base tools + system prompt used
    }

    // Test: resolve with router (append mode)
    #[tokio::test]
    async fn test_resolve_with_router_append() {
        // Setup mock with mode (append_to_agent_system_prompt = true)
        // Mock router response: {"mode": "coding"}
        // Assert: system_prompt = agent + "\n\n" + mode
        // Assert: tools = agent_tools ∪ mode_tools
    }

    // Test: resolve with context_hint
    #[tokio::test]
    async fn test_resolve_with_context_hint() {
        // Setup with context_hint = "Previous conversation..."
        // Assert: routing prompt includes "## Context:"
    }

    // Test: fallback on invalid JSON
    #[tokio::test]
    async fn test_fallback_invalid_json() {
        // Mock router response: "The best mode is coding"
        // Assert: returns first mode (fallback)
    }

    // Test: fallback on unknown mode_key
    #[tokio::test]
    async fn test_fallback_unknown_mode() {
        // Mock router response: {"mode": "nonexistent"}
        // Assert: returns first mode (fallback)
    }

    // Test: tool deduplication
    #[tokio::test]
    async fn test_tool_deduplication() {
        // Agent tools: ["read_file", "write_file"]
        // Mode tools: ["write_file", "run_tests"]
        // Assert: final = ["read_file", "write_file", "run_tests"]
    }
}
