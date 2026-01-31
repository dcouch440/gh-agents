//! Repo indexer — walks source files and calls Haiku to build summaries + symbol maps.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::time::SystemTime;

use futures::stream::{self, StreamExt};
use tokio::fs;

use crate::llm::{AnthropicClient, AnthropicConfig, LLMProvider, LLMRequest, Message as LlmMessage};

use super::{FileEntry, RepoIndex, Symbol};

/// File extensions we index.
const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "rb", "sql",
    "toml", "yaml", "yml", "json", "md", "txt", "sh", "css", "html",
];

/// Directories to skip during walk.
const SKIP_DIRS: &[&str] = &[
    ".git", "target", "node_modules", "dist", "build", ".next",
    "__pycache__", ".venv", "vendor",
];

/// Max file size to index (100KB).
const MAX_FILE_SIZE: u64 = 100_000;

/// Max concurrent Haiku calls during indexing.
const CONCURRENCY: usize = 10;

/// Build the full repo index from scratch.
pub async fn build_index(project_root: &Path) -> RepoIndex {
    let files = collect_source_files(project_root).await;
    tracing::info!("Indexing {} source files", files.len());

    let entries: Vec<FileEntry> = stream::iter(files)
        .map(|path| {
            let root = project_root.to_path_buf();
            async move { index_file(&root, &path).await }
        })
        .buffer_unordered(CONCURRENCY)
        .filter_map(|e| async { e })
        .collect()
        .await;

    build_index_from_entries(entries)
}

/// Incrementally update the index — re-index only changed/new files, remove deleted.
pub async fn update_index(index: &mut RepoIndex, project_root: &Path) {
    let current_files = collect_source_files(project_root).await;
    let current_set: std::collections::HashSet<String> =
        current_files.iter().map(|p| p.to_string_lossy().to_string()).collect();

    // Remove deleted files
    let to_remove: Vec<String> = index
        .files
        .keys()
        .filter(|p| {
            let full = project_root.join(p);
            !current_set.contains(&full.to_string_lossy().to_string())
        })
        .cloned()
        .collect();
    for path in &to_remove {
        index.files.remove(path);
    }

    // Find changed/new files
    let mut to_reindex = Vec::new();
    for path in &current_files {
        let rel = path.strip_prefix(project_root).unwrap_or(path);
        let rel_str = rel.to_string_lossy().to_string();
        let mtime = fs::metadata(path).await.ok().and_then(|m| m.modified().ok());
        let needs_update = match (index.files.get(&rel_str), mtime) {
            (Some(existing), Some(mtime)) => mtime > existing.last_modified,
            (None, _) => true,
            _ => false,
        };
        if needs_update {
            to_reindex.push(path.clone());
        }
    }

    if to_reindex.is_empty() && to_remove.is_empty() {
        return;
    }

    tracing::info!(
        "Index update: {} new/changed, {} removed",
        to_reindex.len(),
        to_remove.len()
    );

    let new_entries: Vec<FileEntry> = stream::iter(to_reindex)
        .map(|path| {
            let root = project_root.to_path_buf();
            async move { index_file(&root, &path).await }
        })
        .buffer_unordered(CONCURRENCY)
        .filter_map(|e| async { e })
        .collect()
        .await;

    // Merge new entries
    for entry in new_entries {
        index.files.insert(entry.path.clone(), entry);
    }

    // Rebuild derived structures
    rebuild_derived(index);
}

/// Re-index a single file (for post-write hooks).
pub async fn reindex_single_file(
    index: &mut RepoIndex,
    project_root: &Path,
    file_path: &Path,
) {
    if let Some(entry) = index_file(project_root, file_path).await {
        // Remove old symbols for this path
        let old_path = entry.path.clone();
        for paths in index.symbol_map.values_mut() {
            paths.retain(|p| p != &old_path);
        }
        // Add new symbols
        for sym in &entry.symbols {
            index
                .symbol_map
                .entry(sym.name.to_lowercase())
                .or_default()
                .push(entry.path.clone());
        }
        index.files.insert(entry.path.clone(), entry);
        // Rebuild tree summary
        index.tree_summary = build_tree_summary(&index.files);
    }
}

/// Collect all source file paths under project_root.
async fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries = match fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                if !SKIP_DIRS.contains(&name.as_str()) && !name.starts_with('.') {
                    stack.push(path);
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if SOURCE_EXTENSIONS.contains(&ext) {
                    if let Ok(meta) = fs::metadata(&path).await {
                        if meta.len() <= MAX_FILE_SIZE {
                            files.push(path);
                        }
                    }
                }
            }
        }
    }
    files
}

/// Index a single file: read content, call Haiku for summary + symbols.
async fn index_file(project_root: &Path, path: &Path) -> Option<FileEntry> {
    let content = fs::read_to_string(path).await.ok()?;
    let mtime = fs::metadata(path).await.ok()?.modified().ok()?;
    let size = content.len() as u64;
    let rel = path.strip_prefix(project_root).unwrap_or(path);
    let rel_str = rel.to_string_lossy().to_string();

    let (summary, symbols) = haiku_index_file(&rel_str, &content).await;

    Some(FileEntry {
        path: rel_str,
        summary,
        symbols,
        size_bytes: size,
        last_modified: mtime,
    })
}

/// Call Haiku to extract summary and symbols from a file.
async fn haiku_index_file(path: &str, content: &str) -> (String, Vec<Symbol>) {
    let config = match AnthropicConfig::from_env() {
        Ok(c) => c,
        Err(_) => return (first_line_summary(content), vec![]),
    };
    let client = match AnthropicClient::new(config) {
        Ok(c) => c,
        Err(_) => return (first_line_summary(content), vec![]),
    };

    // Truncate to 4000 chars for Haiku
    let truncated: String = content.chars().take(4000).collect();
    let prompt = format!(
        "File: {}\n\n```\n{}\n```\n\nReturn JSON only, no markdown:\n{{\"summary\": \"1-2 sentence description\", \"symbols\": [{{\"name\": \"...\", \"kind\": \"Struct|Function|Trait|Enum|Mod|Const|Impl|Type\", \"line\": N}}]}}",
        path, truncated
    );

    let request = LLMRequest::new(
        "claude-haiku-4-20250514",
        vec![LlmMessage::user(prompt)],
    )
    .with_system("You analyze source files and return structured JSON with a brief summary and a list of key symbol definitions. Return ONLY valid JSON, no explanation.")
    .with_max_tokens(512);

    match client.send_message(request).await {
        Ok(resp) => parse_index_response(&resp.content),
        Err(e) => {
            tracing::debug!("Haiku index failed for {}: {}", path, e);
            (first_line_summary(content), vec![])
        }
    }
}

/// Parse Haiku's JSON response into (summary, symbols).
fn parse_index_response(response: &str) -> (String, Vec<Symbol>) {
    // Try to extract JSON from response (might have markdown fences)
    let json_str = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(val) => {
            let summary = val["summary"].as_str().unwrap_or("").to_string();
            let symbols = val["symbols"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| {
                            Some(Symbol {
                                name: s["name"].as_str()?.to_string(),
                                kind: s["kind"].as_str().unwrap_or("Function").to_string(),
                                line: s["line"].as_u64().unwrap_or(0) as u32,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            (summary, symbols)
        }
        Err(_) => (response.chars().take(100).collect(), vec![]),
    }
}

/// Fallback: use first non-empty line as summary.
fn first_line_summary(content: &str) -> String {
    content
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("(empty file)")
        .chars()
        .take(100)
        .collect()
}

fn build_index_from_entries(entries: Vec<FileEntry>) -> RepoIndex {
    let mut files = HashMap::new();
    let mut symbol_map: HashMap<String, Vec<String>> = HashMap::new();

    for entry in entries {
        for sym in &entry.symbols {
            symbol_map
                .entry(sym.name.to_lowercase())
                .or_default()
                .push(entry.path.clone());
        }
        files.insert(entry.path.clone(), entry);
    }

    let tree_summary = build_tree_summary(&files);

    RepoIndex {
        files,
        symbol_map,
        tree_summary,
        ready: true,
    }
}

fn rebuild_derived(index: &mut RepoIndex) {
    index.symbol_map.clear();
    for entry in index.files.values() {
        for sym in &entry.symbols {
            index
                .symbol_map
                .entry(sym.name.to_lowercase())
                .or_default()
                .push(entry.path.clone());
        }
    }
    index.tree_summary = build_tree_summary(&index.files);
}

fn build_tree_summary(files: &HashMap<String, FileEntry>) -> String {
    let mut paths: Vec<&String> = files.keys().collect();
    paths.sort();
    paths
        .iter()
        .map(|p| {
            let summary = &files[*p].summary;
            let short: String = summary.chars().take(80).collect();
            format!("{}: {}", p, short)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_json_response() {
        let resp = r#"{"summary": "Entry point for the app", "symbols": [{"name": "main", "kind": "Function", "line": 5}]}"#;
        let (summary, symbols) = parse_index_response(resp);
        assert_eq!(summary, "Entry point for the app");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "main");
        assert_eq!(symbols[0].line, 5);
    }

    #[test]
    fn parse_json_with_fences() {
        let resp = "```json\n{\"summary\": \"Test\", \"symbols\": []}\n```";
        let (summary, symbols) = parse_index_response(resp);
        assert_eq!(summary, "Test");
        assert!(symbols.is_empty());
    }

    #[test]
    fn parse_invalid_json_fallback() {
        let (summary, symbols) = parse_index_response("not json at all");
        assert!(!summary.is_empty());
        assert!(symbols.is_empty());
    }

    #[test]
    fn first_line_summary_works() {
        assert_eq!(first_line_summary("// Hello world\nfn main() {}"), "// Hello world");
        assert_eq!(first_line_summary("\n\nfn main() {}"), "fn main() {}");
        assert_eq!(first_line_summary(""), "(empty file)");
    }

    #[test]
    fn build_tree_summary_sorted() {
        let mut files = HashMap::new();
        files.insert("b.rs".into(), FileEntry {
            path: "b.rs".into(),
            summary: "B file".into(),
            symbols: vec![],
            size_bytes: 10,
            last_modified: SystemTime::now(),
        });
        files.insert("a.rs".into(), FileEntry {
            path: "a.rs".into(),
            summary: "A file".into(),
            symbols: vec![],
            size_bytes: 10,
            last_modified: SystemTime::now(),
        });
        let tree = build_tree_summary(&files);
        assert!(tree.starts_with("a.rs:"));
        assert!(tree.contains("b.rs:"));
    }

    #[test]
    fn build_index_from_entries_ready() {
        let entries = vec![FileEntry {
            path: "src/main.rs".into(),
            summary: "Entry".into(),
            symbols: vec![Symbol { name: "main".into(), kind: "Function".into(), line: 1 }],
            size_bytes: 50,
            last_modified: SystemTime::now(),
        }];
        let idx = build_index_from_entries(entries);
        assert!(idx.ready);
        assert_eq!(idx.files.len(), 1);
        assert!(idx.symbol_map.contains_key("main"));
    }
}
