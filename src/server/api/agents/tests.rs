//! Tests for agent management endpoints

#[cfg(test)]
mod tests {
    use crate::server::api::agents::*;

    fn test_agent_response() -> AgentResponse {
        AgentResponse {
            id: "agent-1".to_string(),
            name: "Test Agent".to_string(),
            system_prompt: "You are a test agent".to_string(),
            persona_style: "casual".to_string(),
            model_provider: "anthropic".to_string(),
            model_id: "claude-sonnet-4-20250514".to_string(),
            model_max_tokens: 4096,
            model_temperature: 0.7,
            status: "idle".to_string(),
            version: 1,
        }
    }

    #[test]
    fn agent_pool_stats_serializes() {
        let stats = AgentPoolStats { total: 6, available: 5, max: 12 };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total\""));
        assert!(json.contains("\"available\""));
        assert!(json.contains("\"max\""));
    }

    #[test]
    fn agents_list_response_serializes() {
        let response = AgentsListResponse {
            agents: vec![test_agent_response()],
            stats: AgentPoolStats { total: 1, available: 1, max: 12 },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"agent-1\""));
        assert!(json.contains("\"total\""));
    }

    #[test]
    fn agent_response_serializes_all_fields() {
        let response = test_agent_response();
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"Test Agent\""));
        assert!(json.contains("\"model_provider\":\"anthropic\""));
        assert!(json.contains("\"model_max_tokens\":4096"));
    }
}
