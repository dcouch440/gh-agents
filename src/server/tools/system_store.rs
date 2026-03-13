//! Implicit store tools for workforce agents.
//!
//! `store_read_file` and `store_write_file` are available to every workforce
//! agent automatically — they don't need to be assigned by the designer.
//! Agents see paths like `.system/artifacts/research.md`; the handlers strip
//! the `.system/` prefix before calling the store service.

use serde_json::Value;
use uuid::Uuid;

use crate::db::traits::SystemFileRepo;
use crate::llm::Tool;
use crate::server::services::system_store::{s3::S3Backend, store};

/// Tool definition for reading files from the system store.
pub fn store_read_file_tool() -> Tool {
    Tool {
        name: "store_read_file".to_string(),
        description: "Read a file from the system store. Returns the file content as text."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path in the system store (e.g. .system/artifacts/research.md)"
                }
            },
            "required": ["path"]
        }),
    }
}

/// Tool definition for writing files to the system store.
pub fn store_write_file_tool() -> Tool {
    Tool {
        name: "store_write_file".to_string(),
        description: "Write a file to the system store. Provide a description so downstream agents can discover the file in their manifest.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path (e.g. .system/artifacts/findings.md)"
                },
                "content": {
                    "type": "string",
                    "description": "File content to write"
                },
                "description": {
                    "type": "string",
                    "description": "Brief description of the file for downstream agents"
                }
            },
            "required": ["path", "content"]
        }),
    }
}

/// Normalize a user-facing path by stripping the `.system/` prefix.
fn normalize_path(path: &str) -> &str {
    path.strip_prefix(".system/").unwrap_or(path)
}

/// Execute a store_read_file tool call.
pub async fn execute_store_read_file(
    input: &Value,
    s3: &S3Backend,
    repo: &dyn SystemFileRepo,
    workflow_id: Uuid,
) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => normalize_path(p),
        None => return serde_json::json!({"error": "missing required parameter: path"}),
    };

    match store::read_file(s3, repo, workflow_id, path).await {
        Ok((bytes, _meta)) => {
            let content = String::from_utf8_lossy(&bytes).to_string();
            serde_json::json!({ "content": content })
        }
        Err(e) => serde_json::json!({"error": e.to_string()}),
    }
}

/// Execute a store_write_file tool call.
pub async fn execute_store_write_file(
    input: &Value,
    s3: &S3Backend,
    repo: &dyn SystemFileRepo,
    workflow_id: Uuid,
    step_id: Uuid,
    agent_name: Option<&str>,
) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => normalize_path(p),
        None => return serde_json::json!({"error": "missing required parameter: path"}),
    };
    let content = match input["content"].as_str() {
        Some(c) => c,
        None => return serde_json::json!({"error": "missing required parameter: content"}),
    };
    let description = input["description"].as_str().unwrap_or("").to_string();

    match store::write_file(
        s3,
        repo,
        store::WriteFileInput {
            workflow_id,
            path: path.to_string(),
            content: content.as_bytes().to_vec(),
            media_type: String::new(), // auto-inferred from extension
            description,
            tags: vec![],
            produced_by: Some(step_id),
            produced_by_agent: agent_name.map(|s| s.to_string()),
        },
    )
    .await
    {
        Ok(row) => serde_json::json!({
            "status": "written",
            "path": format!(".system/{}", row.path),
            "version": row.version,
        }),
        Err(e) => serde_json::json!({"error": e.to_string()}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_strips_prefix() {
        assert_eq!(
            normalize_path(".system/artifacts/foo.md"),
            "artifacts/foo.md"
        );
        assert_eq!(normalize_path("artifacts/foo.md"), "artifacts/foo.md");
        assert_eq!(
            normalize_path(".system/design/agents/x.json"),
            "design/agents/x.json"
        );
    }

    #[test]
    fn tool_definitions_have_required_fields() {
        let read = store_read_file_tool();
        assert_eq!(read.name, "store_read_file");
        assert!(!read.description.is_empty());

        let write = store_write_file_tool();
        assert_eq!(write.name, "store_write_file");
        assert!(!write.description.is_empty());
    }
}
