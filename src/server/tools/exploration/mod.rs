//! Codebase exploration tool handlers.
//!
//! Read-only tools for navigating the project: reading files,
//! listing directories, searching content, and a reasoning scratchpad.

use serde_json::{json, Value};

use super::haiku::haiku_read_file;

mod tests;

pub(crate) async fn execute_read_file(input: &Value) -> Value {
    let Some(path_str) = input["path"].as_str() else {
        return json!({ "error": "Missing required parameter: path" });
    };
    let focus = input["focus"].as_str();

    // Resolve relative to current working directory (project root)
    let cwd = std::env::current_dir().unwrap_or_default();
    let file_path = cwd.join(path_str);

    // Basic safety: don't escape the project root
    match file_path.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&cwd) {
                return json!({ "error": "Path is outside the project directory" });
            }
            match tokio::fs::read_to_string(&canonical).await {
                Ok(content) => {
                    let size_bytes = content.len();
                    let line_count = content.lines().count();

                    // Small files: return directly
                    if content.len() <= crate::constants::TRUNCATE_SMALL_FILE {
                        return json!({
                            "path": path_str,
                            "content": content,
                            "line_count": line_count,
                            "size_bytes": size_bytes,
                            "summarized": false
                        });
                    }

                    // Large files: summarize with Haiku
                    let truncated_for_haiku: String = content
                        .chars()
                        .take(crate::constants::TRUNCATE_SUMMARIZE_INPUT)
                        .collect();
                    let focus_instruction = match focus {
                        Some(f) => format!("Focus on: {}. Extract the most relevant code sections, function signatures, and logic related to this focus area.", f),
                        None => "Extract the key structures, function signatures, imports, and overall purpose of this file.".to_string(),
                    };

                    let prompt = format!(
                        "File: {} ({} lines, {} bytes)\n\n{}\n\n---\n{}",
                        path_str, line_count, size_bytes, focus_instruction, truncated_for_haiku
                    );

                    match haiku_read_file(&prompt).await {
                        Some(summary) => json!({
                            "path": path_str,
                            "summary": summary,
                            "line_count": line_count,
                            "size_bytes": size_bytes,
                            "summarized": true
                        }),
                        None => {
                            // Haiku failed — fall back to truncated content
                            let fallback: String = content
                                .chars()
                                .take(crate::constants::TRUNCATE_SMALL_FILE)
                                .collect();
                            json!({
                                "path": path_str,
                                "content": fallback,
                                "line_count": line_count,
                                "size_bytes": size_bytes,
                                "summarized": false,
                                "truncated": true
                            })
                        }
                    }
                }
                Err(e) => json!({ "error": format!("Could not read file: {}", e) }),
            }
        }
        Err(e) => json!({ "error": format!("File not found or inaccessible: {}", e) }),
    }
}

pub(crate) async fn execute_list_files(input: &Value) -> Value {
    let path_str = input["path"].as_str().unwrap_or(".");

    let cwd = std::env::current_dir().unwrap_or_default();
    let dir_path = if path_str.is_empty() || path_str == "." {
        cwd.clone()
    } else {
        cwd.join(path_str)
    };

    match dir_path.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&cwd) {
                return json!({ "error": "Path is outside the project directory" });
            }
            match tokio::fs::read_dir(&canonical).await {
                Ok(mut entries) => {
                    let mut files = Vec::new();
                    let mut dirs = Vec::new();
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // Skip hidden files/dirs
                        if name.starts_with('.') {
                            continue;
                        }
                        if let Ok(ft) = entry.file_type().await {
                            if ft.is_dir() {
                                dirs.push(format!("{}/", name));
                            } else {
                                files.push(name);
                            }
                        }
                    }
                    dirs.sort();
                    files.sort();
                    json!({
                        "path": path_str,
                        "directories": dirs,
                        "files": files
                    })
                }
                Err(e) => json!({ "error": format!("Could not list directory: {}", e) }),
            }
        }
        Err(e) => json!({ "error": format!("Directory not found: {}", e) }),
    }
}

pub(crate) async fn execute_search_files(input: &Value) -> Value {
    let Some(pattern) = input["pattern"].as_str() else {
        return json!({ "error": "Missing required parameter: pattern" });
    };
    let path_str = input["path"].as_str().unwrap_or(".");
    let max_results = input["max_results"]
        .as_u64()
        .unwrap_or(crate::constants::DEFAULT_SEARCH_RESULTS as u64) as usize;

    let cwd = std::env::current_dir().unwrap_or_default();
    let search_dir = if path_str.is_empty() || path_str == "." {
        cwd.clone()
    } else {
        cwd.join(path_str)
    };

    // Validate path
    match search_dir.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&cwd) {
                return json!({ "error": "Path is outside the project directory" });
            }

            // Use grep -rn for search
            let output = tokio::process::Command::new("grep")
                .args([
                    "-rn",
                    "--include=*.rs",
                    "--include=*.ts",
                    "--include=*.tsx",
                    "--include=*.js",
                    "--include=*.json",
                    "--include=*.toml",
                    "--include=*.sql",
                    "--include=*.md",
                    "--include=*.txt",
                    "--include=*.css",
                    "--include=*.html",
                    "-m",
                    &(max_results * 2).to_string(), // overfetch for filtering
                    pattern,
                ])
                .arg(&canonical)
                .output()
                .await;

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let matches: Vec<Value> = stdout
                        .lines()
                        .take(max_results)
                        .filter_map(|line| {
                            // Format: /abs/path:line_num:content
                            let rest = line.strip_prefix(canonical.to_str()?)?;
                            let rest = rest.strip_prefix('/')?;
                            let mut parts = rest.splitn(3, ':');
                            let file = parts.next()?;
                            let line_num = parts.next()?;
                            let text = parts.next().unwrap_or("").trim();
                            Some(json!({
                                "file": file,
                                "line": line_num.parse::<u64>().unwrap_or(0),
                                "text": &text[..text.len().min(200)]
                            }))
                        })
                        .collect();

                    let total = stdout.lines().count();
                    json!({
                        "pattern": pattern,
                        "matches": matches,
                        "total_matches": total,
                        "truncated": total > max_results
                    })
                }
                Err(e) => json!({ "error": format!("Search failed: {}", e) }),
            }
        }
        Err(e) => json!({ "error": format!("Directory not found: {}", e) }),
    }
}

/// The think tool is a no-op — it returns the agent's reasoning back to it.
/// This gives the model a scratchpad to reason step-by-step before acting.
pub(crate) fn execute_think(input: &Value) -> Value {
    let thought = input["thought"].as_str().unwrap_or("");
    json!({ "thought_recorded": true, "length": thought.len() })
}
