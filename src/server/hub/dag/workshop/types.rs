//! Workshop result types.

use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Result from executing a single workshop step.
pub struct WorkshopStepResult {
    pub step_id: Uuid,
    pub status: String,
    pub output: Option<JsonValue>,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f32,
    pub duration_ms: u64,
}
