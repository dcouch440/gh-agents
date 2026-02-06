//! Cavernous routing types for document-based dynamic execution

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Document summary for routing config search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSummary {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub score: f64,
}

/// Routing analysis stored in agent_executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingAnalysis {
    pub search_query: String,
    pub documents_found: Vec<DocumentSummary>,
    pub selected_document_id: Uuid,
    pub reasoning: String,
    pub collaborative_selection: bool,
}

/// Routing configuration document (parsed from document content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfigDocument {
    pub strategy_name: String,
    pub version: Option<String>,
    pub description: String,
    pub capabilities_required: Vec<String>,
    pub complexity_level: Option<String>,
    pub subtasks: Vec<Subtask>,
    pub aggregation_mode: String,
    pub max_parallel: usize,
    pub timeout_minutes: Option<u64>,
    pub cost_limit_usd: Option<f64>,
}

/// Individual subtask in routing config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub id: String,
    pub task_name: String,
    pub agent_id: Uuid,
    pub tools: Vec<String>,
    pub prompt_template: String,
    pub depends_on: Vec<String>,
    pub input_mapping: HashMap<String, String>,
    pub output_schema: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_config_parses() {
        let json = r#"{
            "strategy_name": "test",
            "description": "Test strategy",
            "capabilities_required": ["file_write"],
            "subtasks": [],
            "aggregation_mode": "final_output",
            "max_parallel": 1
        }"#;

        let config: RoutingConfigDocument = serde_json::from_str(json).unwrap();
        assert_eq!(config.strategy_name, "test");
    }

    #[test]
    fn subtask_with_dependencies() {
        let subtask = Subtask {
            id: "task1".into(),
            task_name: "Test Task".into(),
            agent_id: Uuid::new_v4(),
            tools: vec!["file_write".into()],
            prompt_template: "Do something".into(),
            depends_on: vec!["task0".into()],
            input_mapping: HashMap::new(),
            output_schema: None,
        };

        assert_eq!(subtask.depends_on.len(), 1);
    }
}
