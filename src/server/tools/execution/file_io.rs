//! Shared file I/O abstraction for local and container execution.

use serde_json::{json, Value};

use crate::execution::{ContainerHandle, ExecutionContext, FileOps};

/// Abstraction over file read/write for local and container execution.
///
/// Allows `edit_file_core` to work identically for both local FileOps
/// and container-based file operations.
#[async_trait::async_trait]
pub(super) trait FileIO: Send + Sync {
    async fn read(&self, path: &str) -> Result<String, String>;
    async fn write(&self, path: &str, content: &str) -> Result<(), String>;
}

/// Local filesystem implementation of FileIO.
pub(super) struct LocalFileIO<'a> {
    pub file_ops: FileOps,
    pub ctx: &'a ExecutionContext,
}

#[async_trait::async_trait]
impl FileIO for LocalFileIO<'_> {
    async fn read(&self, path: &str) -> Result<String, String> {
        let full_path = self.ctx.project_root.join(path);
        self.file_ops
            .read_file(&full_path)
            .await
            .map_err(|e| e.to_string())
    }
    async fn write(&self, path: &str, content: &str) -> Result<(), String> {
        let full_path = self.ctx.project_root.join(path);
        self.file_ops
            .write_file(&full_path, content)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Container-based implementation of FileIO.
pub(super) struct ContainerFileIO<'a> {
    pub handle: &'a ContainerHandle,
}

#[async_trait::async_trait]
impl FileIO for ContainerFileIO<'_> {
    async fn read(&self, path: &str) -> Result<String, String> {
        self.handle.read_file(path).await.map_err(|e| e.to_string())
    }
    async fn write(&self, path: &str, content: &str) -> Result<(), String> {
        self.handle
            .write_file(path, content)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Core edit_file logic shared between local and container execution.
///
/// Reads the file, performs the replacement (or append), writes back,
/// and returns a JSON result with preview context.
pub(super) async fn edit_file_core(
    path: &str,
    old_string: &str,
    new_string: &str,
    io: &dyn FileIO,
) -> Value {
    // Handle append mode: empty old_string means append to end
    if old_string.is_empty() {
        let existing = io.read(path).await.unwrap_or_default();
        let new_content = if existing.is_empty() {
            new_string.to_string()
        } else if existing.ends_with('\n') {
            format!("{}{}", existing, new_string)
        } else {
            format!("{}\n{}", existing, new_string)
        };
        return match io.write(path, &new_content).await {
            Ok(()) => json!({ "success": true, "path": path, "action": "appended" }),
            Err(e) => json!({ "error": e }),
        };
    }

    // Read the existing file
    let content = match io.read(path).await {
        Ok(c) => c,
        Err(e) => return json!({ "error": e }),
    };

    // Count occurrences
    let matches: Vec<_> = content.match_indices(old_string).collect();

    if matches.is_empty() {
        return json!({
            "error": format!("old_string not found in {}", path),
            "hint": "Check for exact whitespace and newline matches. Use read_file to see the current content."
        });
    }

    if matches.len() > 1 {
        return json!({
            "error": format!("old_string matches {} locations in {}. Add surrounding context to make it unique.", matches.len(), path),
            "match_count": matches.len()
        });
    }

    // Exactly one match — perform the replacement
    let byte_offset = matches[0].0;
    let new_content = format!(
        "{}{}{}",
        &content[..byte_offset],
        new_string,
        &content[byte_offset + old_string.len()..]
    );

    match io.write(path, &new_content).await {
        Ok(()) => {
            let line_start = content[..byte_offset].matches('\n').count() + 1;
            let line_end = line_start + new_string.matches('\n').count();

            let new_lines: Vec<&str> = new_content.lines().collect();
            let preview_start = line_start.saturating_sub(2);
            let preview_end = (line_end + 2).min(new_lines.len());
            let preview: Vec<String> = new_lines[preview_start..preview_end]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>4} | {}", preview_start + i + 1, line))
                .collect();

            json!({
                "success": true,
                "path": path,
                "line_start": line_start,
                "line_end": line_end,
                "preview": preview.join("\n")
            })
        }
        Err(e) => json!({ "error": e }),
    }
}
