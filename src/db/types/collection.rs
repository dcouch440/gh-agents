use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for workflow collections (DAG of workflows).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowCollectionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub execution_mode: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for collection workflows (which workflows belong to a collection).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CollectionWorkflowRow {
    pub collection_id: Uuid,
    pub workflow_id: Uuid,
    pub display_order: i32,
    pub execution_mode: Option<String>,
}

/// Row type for collection workflow edges (DAG edges between workflows).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CollectionWorkflowEdgeRow {
    pub from_workflow_id: Uuid,
    pub to_workflow_id: Uuid,
    pub collection_id: Uuid,
}

/// Row type for collection runs (execution tracking).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CollectionRunRow {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}
