//! Room state types for structured agent collaboration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Room state accumulator (not persisted, runtime only)
#[derive(Debug, Clone, Default)]
pub struct RoomState {
    outputs: HashMap<String, RoomExecutionOutput>,  // Latest by output_name
    all_outputs: Vec<RoomExecutionOutput>,
}

impl RoomState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_output(&mut self, output: RoomExecutionOutput) {
        self.outputs.insert(output.output_name.clone(), output.clone());
        self.all_outputs.push(output);
    }

    pub fn get_output(&self, name: &str) -> Option<&RoomExecutionOutput> {
        self.outputs.get(name)
    }

    pub fn get_outputs_by_schema(&self, schema_id: Uuid) -> Vec<&RoomExecutionOutput> {
        self.all_outputs
            .iter()
            .filter(|o| o.schema_id == Some(schema_id))
            .collect()
    }

    pub fn all_outputs(&self) -> &[RoomExecutionOutput] {
        &self.all_outputs
    }
}

/// Room execution output (simplified for domain logic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomExecutionOutput {
    pub id: Uuid,
    pub room_session_id: Uuid,
    pub agent_execution_id: Uuid,
    pub agent_id: Uuid,
    pub turn_number: i32,
    pub output_name: String,
    pub structured_output: serde_json::Value,
    pub schema_id: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_state_accumulation() {
        let mut state = RoomState::new();

        let output1 = RoomExecutionOutput {
            id: Uuid::new_v4(),
            room_session_id: Uuid::new_v4(),
            agent_execution_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            turn_number: 1,
            output_name: "analysis".into(),
            structured_output: serde_json::json!({"key": "value"}),
            schema_id: None,
        };

        state.add_output(output1.clone());

        assert_eq!(state.all_outputs().len(), 1);
        assert!(state.get_output("analysis").is_some());
    }

    #[test]
    fn get_outputs_by_schema() {
        let mut state = RoomState::new();
        let schema_id = Uuid::new_v4();

        let output1 = RoomExecutionOutput {
            id: Uuid::new_v4(),
            room_session_id: Uuid::new_v4(),
            agent_execution_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            turn_number: 1,
            output_name: "output1".into(),
            structured_output: serde_json::json!({}),
            schema_id: Some(schema_id),
        };

        let output2 = RoomExecutionOutput {
            id: Uuid::new_v4(),
            room_session_id: Uuid::new_v4(),
            agent_execution_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            turn_number: 2,
            output_name: "output2".into(),
            structured_output: serde_json::json!({}),
            schema_id: None,
        };

        state.add_output(output1);
        state.add_output(output2);

        let filtered = state.get_outputs_by_schema(schema_id);
        assert_eq!(filtered.len(), 1);
    }
}
