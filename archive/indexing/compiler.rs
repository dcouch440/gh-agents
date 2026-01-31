//! Context compiler — selects relevant files from the repo index for a given task.

use std::collections::HashMap;
use std::path::Path;

use crate::llm::{
    AnthropicClient, AnthropicConfig, LLMProvider, LLMRequest, Message as LlmMessage,
};

use super::RepoIndex;

/// Maximum number of files to pre-load into context.
const MAX_FILES: usize = 5;

/// Maximum file content size to include (50KB).
const MAX_CONTENT_SIZE: usize = 50_000;

/// Compile relevant context from the repo index for a task.
pub async fn compile_context(
    index: &RepoIndex,
    title: &str,
    description: &str,
    project_root: &Path,
) -> super::CompiledContext {
    // Step 1: Score files by keyword matching
    let keywords = extract_keywords(title, description);
    let mut scores: HashMap<&str, i32> = HashMap::new();

    // Symbol matches (+3)
    for kw in &keywords {
        if let Some(paths) = index.symbol_map.get(kw.as_str()) {
            for path in paths {
                *scores.entry(path.as_str()).or_default() += 3;
            }
        }
    }

    // Path substring matches (+2)
    for (path, _) in &index.files {
        for kw in &keywords {
            if kw.len() >= 3 && path.to_lowercase().contains(kw.as_str()) {
                *scores.entry(path.as_str()).or_default() += 2;
            }
        }
    }

    // Summary substring matches (+1)
    for (path, entry) in &index.files {
        let summary_lower = entry.summary.to_lowercase();
        for kw in &keywords {
            if kw.len() >= 3 && summary_lower.contains(kw.as_str()) {
                *scores.entry(path.as_str()).or_default() += 1;
            }
        }
    }

    if scores.is_empty() {
        return super::CompiledContext {
            briefing: String::new(),
            relevant_files: vec![],
        };
    }

    // Step 2: Rank candidates
    let mut candidates: Vec<(&str, i32)> = scores.into_iter().collect();
    candidates.sort_by(|a, b| b.1.cmp(&a.1));

    // If too many candidates, ask Haiku to rank
    let top_paths: Vec<String> = if candidates.len() > 10 {
        let top_20: Vec<(&str, &str)> = candidates
            .iter()
            .take(20)
            .filter_map(|(p, _)| index.files.get(*p).map(|e| (*p, e.summary.as_str())))
            .collect();
        haiku_rank_files(title, description, &top_20)
            .await
            .unwrap_or_else(|| {
                candidates
                    .iter()
                    .take(MAX_FILES)
                    .map(|(p, _)| p.to_string())
                    .collect()
            })
    } else {
        candidates
            .iter()
            .take(MAX_FILES)
            .map(|(p, _)| p.to_string())
            .collect()
    };

    // Step 3: Load file contents
    let mut relevant_files = Vec::new();
    for path in &top_paths {
        let full_path = project_root.join(path);
        if let Ok(content) = tokio::fs::read_to_string(&full_path).await {
            let truncated: String = content.chars().take(MAX_CONTENT_SIZE).collect();
            relevant_files.push((path.clone(), truncated));
        }
    }

    // Step 4: Build briefing
    let mut briefing = String::from("## Codebase Briefing\n\n");

    // Relevant file tree subset
    briefing.push_str("### Relevant Files\n");
    for path in &top_paths {
        if let Some(entry) = index.files.get(path.as_str()) {
            briefing.push_str(&format!("- `{}`: {}\n", path, entry.summary));
        }
    }

    // Key symbols
    let mut symbol_lines = Vec::new();
    for path in &top_paths {
        if let Some(entry) = index.files.get(path.as_str()) {
            for sym in &entry.symbols {
                for kw in &keywords {
                    if sym.name.to_lowercase().contains(kw.as_str()) {
                        symbol_lines.push(format!(
                            "- `{}` ({}) at `{}:{}`",
                            sym.name, sym.kind, path, sym.line
                        ));
                        break;
                    }
                }
            }
        }
    }
    if !symbol_lines.is_empty() {
        briefing.push_str("\n### Key Symbols\n");
        for line in &symbol_lines {
            briefing.push_str(line);
            briefing.push('\n');
        }
    }

    super::CompiledContext {
        briefing,
        relevant_files,
    }
}

/// Extract lowercased keywords from title and description.
fn extract_keywords(title: &str, description: &str) -> Vec<String> {
    let combined = format!("{} {}", title, description);
    let stop_words = [
        "the",
        "a",
        "an",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "will",
        "would",
        "could",
        "should",
        "may",
        "might",
        "shall",
        "can",
        "need",
        "dare",
        "ought",
        "used",
        "to",
        "of",
        "in",
        "for",
        "on",
        "with",
        "at",
        "by",
        "from",
        "as",
        "into",
        "through",
        "during",
        "before",
        "after",
        "above",
        "below",
        "between",
        "out",
        "off",
        "over",
        "under",
        "again",
        "further",
        "then",
        "once",
        "here",
        "there",
        "when",
        "where",
        "why",
        "how",
        "all",
        "each",
        "every",
        "both",
        "few",
        "more",
        "most",
        "other",
        "some",
        "such",
        "no",
        "nor",
        "not",
        "only",
        "own",
        "same",
        "so",
        "than",
        "too",
        "very",
        "and",
        "but",
        "or",
        "if",
        "while",
        "because",
        "until",
        "that",
        "this",
        "these",
        "those",
        "it",
        "its",
        "we",
        "they",
        "them",
        "their",
        "what",
        "which",
        "who",
        "whom",
        "add",
        "create",
        "update",
        "fix",
        "implement",
        "change",
        "modify",
        "new",
        "file",
        "code",
    ];
    let stop_set: std::collections::HashSet<&str> = stop_words.iter().copied().collect();

    combined
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() >= 2 && !stop_set.contains(w.as_str()))
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Ask Haiku to rank file candidates by relevance to a task.
async fn haiku_rank_files(
    title: &str,
    description: &str,
    candidates: &[(&str, &str)],
) -> Option<Vec<String>> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let file_list: String = candidates
        .iter()
        .map(|(path, summary)| format!("- {}: {}", path, summary))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Task: {} — {}\n\nFiles:\n{}\n\nReturn a JSON array of the {} most relevant file paths for this task. Example: [\"src/foo.rs\", \"src/bar.rs\"]",
        title, description, file_list, MAX_FILES
    );

    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(prompt)],
    )
    .with_system("You rank source files by relevance to a task. Return ONLY a JSON array of file paths, most relevant first. No explanation.")
    .with_max_tokens(crate::constants::MAX_TOKENS_COMPILER);

    match client.send_message(request).await {
        Ok(resp) => {
            let text = resp.content.trim();
            let json_str = text
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            serde_json::from_str::<Vec<String>>(json_str).ok()
        }
        Err(e) => {
            tracing::debug!("Haiku ranking failed: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_keywords_filters_stop_words() {
        let kws = extract_keywords("Add authentication to the API", "We need to implement auth");
        assert!(kws.contains(&"authentication".to_string()));
        assert!(kws.contains(&"api".to_string()));
        assert!(kws.contains(&"auth".to_string()));
        assert!(!kws.contains(&"the".to_string()));
        assert!(!kws.contains(&"to".to_string()));
        assert!(!kws.contains(&"add".to_string()));
    }

    #[test]
    fn extract_keywords_handles_underscores() {
        let kws = extract_keywords("Fix task_context building", "");
        assert!(kws.contains(&"task_context".to_string()));
    }

    #[tokio::test]
    async fn compile_empty_index_returns_empty() {
        let index = RepoIndex::default();
        let ctx = compile_context(&index, "test", "description", Path::new("/tmp")).await;
        assert!(ctx.briefing.is_empty());
        assert!(ctx.relevant_files.is_empty());
    }

    #[tokio::test]
    async fn compile_scores_symbol_matches() {
        let mut index = RepoIndex::default();
        index.files.insert(
            "src/auth.rs".into(),
            super::super::FileEntry {
                path: "src/auth.rs".into(),
                summary: "Authentication module".into(),
                symbols: vec![super::super::Symbol {
                    name: "AuthService".into(),
                    kind: "Struct".into(),
                    line: 10,
                }],
                size_bytes: 100,
                last_modified: std::time::SystemTime::now(),
            },
        );
        index
            .symbol_map
            .insert("authservice".into(), vec!["src/auth.rs".into()]);
        index.ready = true;

        let ctx = compile_context(
            &index,
            "Fix AuthService login",
            "The login method crashes",
            Path::new("/nonexistent"),
        )
        .await;
        assert!(ctx.briefing.contains("auth.rs"));
    }
}
