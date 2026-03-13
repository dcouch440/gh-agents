use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::db::traits::{SystemFileRepo, UpsertSystemFileInput};
use crate::db::SystemFileRow;

use super::s3::S3Backend;

/// Input for writing a file to the system store.
#[derive(Debug, Clone)]
pub struct WriteFileInput {
    pub workflow_id: Uuid,
    pub path: String,
    pub content: Vec<u8>,
    pub media_type: String,
    pub description: String,
    pub tags: Vec<String>,
    pub produced_by: Option<Uuid>,
    pub produced_by_agent: Option<String>,
}

/// Resolve the S3 object key for a workflow file.
fn s3_key(workflow_id: Uuid, path: &str) -> String {
    format!("workflows/{}/system/{}", workflow_id, path)
}

/// Infer media type from file extension.
fn infer_media_type(path: &str) -> &str {
    match path.rsplit('.').next() {
        Some("json") => "application/json",
        Some("md") => "text/markdown",
        Some("txt") => "text/plain",
        Some("csv") => "text/csv",
        Some("html") => "text/html",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("pdf") => "application/pdf",
        Some("yaml" | "yml") => "application/yaml",
        _ => "application/octet-stream",
    }
}

/// Write a file to the system store (S3 + metadata).
pub async fn write_file(
    s3: &S3Backend,
    repo: &dyn SystemFileRepo,
    input: WriteFileInput,
) -> Result<SystemFileRow> {
    let key = s3_key(input.workflow_id, &input.path);
    let media_type = if input.media_type.is_empty() {
        infer_media_type(&input.path).to_string()
    } else {
        input.media_type.clone()
    };

    s3.write(&key, &input.content, &media_type).await?;

    let row = repo
        .upsert_file(UpsertSystemFileInput {
            workflow_id: input.workflow_id,
            path: input.path,
            media_type,
            description: input.description,
            tags: input.tags,
            produced_by: input.produced_by,
            produced_by_agent: input.produced_by_agent,
            size_bytes: input.content.len() as i64,
        })
        .await?;

    Ok(row)
}

/// Read a file from the system store.
///
/// Returns the raw bytes and the metadata row.
pub async fn read_file(
    s3: &S3Backend,
    repo: &dyn SystemFileRepo,
    workflow_id: Uuid,
    path: &str,
) -> Result<(Vec<u8>, SystemFileRow)> {
    let meta = repo
        .get_file(workflow_id, path)
        .await?
        .ok_or_else(|| anyhow!("system file not found: {path}"))?;

    let key = s3_key(workflow_id, path);
    let bytes = s3.read(&key).await?;

    Ok((bytes, meta))
}

/// Edit a file by find-and-replace. Returns the updated metadata.
pub async fn edit_file(
    s3: &S3Backend,
    repo: &dyn SystemFileRepo,
    workflow_id: Uuid,
    path: &str,
    find: &str,
    replace: &str,
) -> Result<SystemFileRow> {
    let key = s3_key(workflow_id, path);
    let bytes = s3.read(&key).await?;
    let content = String::from_utf8(bytes)?;

    if !content.contains(find) {
        return Err(anyhow!("find string not found in {path}"));
    }

    let new_content = content.replace(find, replace);
    let meta = repo
        .get_file(workflow_id, path)
        .await?
        .ok_or_else(|| anyhow!("system file not found: {path}"))?;

    s3.write(&key, new_content.as_bytes(), &meta.media_type)
        .await?;

    let row = repo
        .upsert_file(UpsertSystemFileInput {
            workflow_id,
            path: path.to_string(),
            media_type: meta.media_type,
            description: meta.description,
            tags: meta.tags,
            produced_by: meta.produced_by,
            produced_by_agent: meta.produced_by_agent,
            size_bytes: new_content.len() as i64,
        })
        .await?;

    Ok(row)
}

/// List files by prefix.
pub async fn list_files(
    repo: &dyn SystemFileRepo,
    workflow_id: Uuid,
    prefix: &str,
) -> Result<Vec<SystemFileRow>> {
    repo.list_files(workflow_id, prefix).await
}

/// Delete a single file (S3 + metadata). Returns true if the file existed.
pub async fn delete_file(
    s3: &S3Backend,
    repo: &dyn SystemFileRepo,
    workflow_id: Uuid,
    path: &str,
) -> Result<bool> {
    let key = s3_key(workflow_id, path);
    s3.delete(&key).await?;
    repo.delete_file(workflow_id, path).await
}

/// Delete all files under a prefix (S3 + metadata). Returns count deleted.
pub async fn delete_prefix(
    s3: &S3Backend,
    repo: &dyn SystemFileRepo,
    workflow_id: Uuid,
    prefix: &str,
) -> Result<u64> {
    let s3_prefix = s3_key(workflow_id, prefix);
    s3.delete_prefix(&s3_prefix).await?;
    repo.delete_by_prefix(workflow_id, prefix).await
}

/// List all files produced by a specific step.
pub async fn artifacts_for_step(
    repo: &dyn SystemFileRepo,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<Vec<SystemFileRow>> {
    repo.list_by_producer(workflow_id, step_id).await
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn s3_key_format() {
        let wf = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        assert_eq!(
            s3_key(wf, "design/agents/scanner.json"),
            "workflows/11111111-1111-1111-1111-111111111111/system/design/agents/scanner.json"
        );
    }

    #[test]
    fn infer_media_type_covers_common_extensions() {
        assert_eq!(infer_media_type("foo.json"), "application/json");
        assert_eq!(infer_media_type("notes.md"), "text/markdown");
        assert_eq!(infer_media_type("data.csv"), "text/csv");
        assert_eq!(infer_media_type("image.png"), "image/png");
        assert_eq!(infer_media_type("unknown.xyz"), "application/octet-stream");
        assert_eq!(infer_media_type("no_extension"), "application/octet-stream");
    }
}
