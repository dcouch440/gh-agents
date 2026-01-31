//! File summarization for large files.
//!
//! Summarizes large files to fit within context budget while
//! preserving important structural information.

use super::manager::estimate_tokens;

/// Summarizes large files to fit in context budget.
pub struct FileSummarizer {
    /// Max tokens before summarization kicks in
    threshold_tokens: usize,
    /// Target size after summarization
    target_tokens: usize,
}

impl FileSummarizer {
    pub fn new(threshold_tokens: usize, target_tokens: usize) -> Self {
        Self {
            threshold_tokens,
            target_tokens,
        }
    }

    /// Get the threshold tokens.
    pub fn threshold_tokens(&self) -> usize {
        self.threshold_tokens
    }

    /// Get the target tokens.
    pub fn target_tokens(&self) -> usize {
        self.target_tokens
    }

    /// Summarize a file if it exceeds the threshold.
    ///
    /// Returns the original content if under threshold, or a summary.
    pub fn summarize_if_needed(&self, content: &str, file_extension: &str) -> SummaryResult {
        let estimated_tokens = estimate_tokens(content);

        if estimated_tokens <= self.threshold_tokens {
            return SummaryResult {
                content: content.to_string(),
                was_summarized: false,
                original_tokens: estimated_tokens,
                summary_tokens: estimated_tokens,
            };
        }

        let summary = self.create_summary(content, file_extension);
        let summary_tokens = estimate_tokens(&summary);

        SummaryResult {
            content: summary,
            was_summarized: true,
            original_tokens: estimated_tokens,
            summary_tokens,
        }
    }

    fn create_summary(&self, content: &str, extension: &str) -> String {
        match extension {
            "rs" => self.summarize_rust(content),
            "py" => self.summarize_python(content),
            "ts" | "js" => self.summarize_javascript(content),
            _ => self.summarize_generic(content),
        }
    }

    fn summarize_rust(&self, content: &str) -> String {
        let mut summary_lines = Vec::new();
        let mut in_impl_block = false;
        let mut brace_depth = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            // Track brace depth for impl blocks
            brace_depth += trimmed.matches('{').count();
            brace_depth = brace_depth.saturating_sub(trimmed.matches('}').count());

            // Keep module declarations
            if trimmed.starts_with("mod ") {
                summary_lines.push(line.to_string());
                continue;
            }

            // Keep use statements
            if trimmed.starts_with("use ") {
                summary_lines.push(line.to_string());
                continue;
            }

            // Keep struct/enum definitions
            if trimmed.starts_with("pub struct ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("enum ")
            {
                summary_lines.push(line.to_string());
                continue;
            }

            // Keep trait definitions
            if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
                summary_lines.push(line.to_string());
                continue;
            }

            // Keep impl headers
            if trimmed.starts_with("impl ") {
                summary_lines.push(line.to_string());
                in_impl_block = true;
                continue;
            }

            // Keep function signatures in impl blocks
            if in_impl_block
                && (trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("fn ")
                    || trimmed.starts_with("pub async fn ")
                    || trimmed.starts_with("async fn "))
            {
                // Extract just the signature, not the body
                if let Some(sig) = self.extract_fn_signature(line) {
                    summary_lines.push(format!("{}  // ...", sig));
                }
                continue;
            }

            // Reset impl block tracking
            if in_impl_block && brace_depth == 0 {
                in_impl_block = false;
                summary_lines.push("}".to_string());
            }
        }

        let summary = summary_lines.join("\n");

        // If still too long, truncate with notice
        if estimate_tokens(&summary) > self.target_tokens {
            self.truncate_with_notice(&summary, self.target_tokens)
        } else {
            format!(
                "// SUMMARIZED (original: {} lines)\n{}",
                content.lines().count(),
                summary
            )
        }
    }

    fn summarize_python(&self, content: &str) -> String {
        let mut summary_lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Keep imports
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                summary_lines.push(line.to_string());
                continue;
            }

            // Keep class definitions
            if trimmed.starts_with("class ") {
                summary_lines.push(line.to_string());
                continue;
            }

            // Keep function definitions (but not body)
            if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                summary_lines.push(format!("{}  # ...", line));
                continue;
            }

            // Keep decorated functions
            if trimmed.starts_with('@') {
                summary_lines.push(line.to_string());
                continue;
            }
        }

        format!(
            "# SUMMARIZED (original: {} lines)\n{}",
            content.lines().count(),
            summary_lines.join("\n")
        )
    }

    fn summarize_javascript(&self, content: &str) -> String {
        let mut summary_lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Keep imports
            if trimmed.starts_with("import ")
                || (trimmed.starts_with("const ") && trimmed.contains(" require("))
            {
                summary_lines.push(line.to_string());
                continue;
            }

            // Keep exports
            if trimmed.starts_with("export ") {
                if let Some(sig) = self.extract_js_export_signature(trimmed) {
                    summary_lines.push(format!("{}  // ...", sig));
                } else {
                    summary_lines.push(line.to_string());
                }
                continue;
            }

            // Keep function declarations
            if trimmed.starts_with("function ") || trimmed.starts_with("async function ") {
                summary_lines.push(format!(
                    "{}  // ...",
                    line.split('{').next().unwrap_or(line)
                ));
                continue;
            }

            // Keep class declarations
            if trimmed.starts_with("class ") {
                summary_lines.push(line.to_string());
                continue;
            }
        }

        format!(
            "// SUMMARIZED (original: {} lines)\n{}",
            content.lines().count(),
            summary_lines.join("\n")
        )
    }

    fn summarize_generic(&self, content: &str) -> String {
        // For unknown formats, take first N lines and last N lines
        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();

        if total <= 50 {
            return content.to_string();
        }

        let head: Vec<&str> = lines.iter().take(20).copied().collect();
        let tail: Vec<&str> = lines.iter().skip(total - 20).copied().collect();

        format!(
            "// SUMMARIZED (original: {} lines)\n{}\n\n// ... ({} lines omitted) ...\n\n{}",
            total,
            head.join("\n"),
            total - 40,
            tail.join("\n")
        )
    }

    fn extract_fn_signature(&self, line: &str) -> Option<String> {
        // Extract up to opening brace or semicolon
        if let Some(brace_pos) = line.find('{') {
            Some(line[..brace_pos].trim().to_string())
        } else if let Some(semi_pos) = line.find(';') {
            Some(line[..=semi_pos].to_string())
        } else {
            Some(line.to_string())
        }
    }

    fn extract_js_export_signature(&self, line: &str) -> Option<String> {
        line.find('{').map(|brace_pos| line[..brace_pos].trim().to_string())
    }

    fn truncate_with_notice(&self, content: &str, max_tokens: usize) -> String {
        let chars_budget = max_tokens * 4; // Rough estimate
        if content.len() <= chars_budget {
            return content.to_string();
        }

        let truncated = &content[..chars_budget.min(content.len())];
        format!("{}\n\n// ... (truncated, original too large)", truncated)
    }
}

#[derive(Debug, Clone)]
pub struct SummaryResult {
    pub content: String,
    pub was_summarized: bool,
    pub original_tokens: usize,
    pub summary_tokens: usize,
}

impl SummaryResult {
    pub fn reduction_ratio(&self) -> f64 {
        if self.original_tokens == 0 {
            1.0
        } else {
            self.summary_tokens as f64 / self.original_tokens as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarizer_below_threshold() {
        let summarizer = FileSummarizer::new(1000, 500);
        let content = "fn main() {}\n";

        let result = summarizer.summarize_if_needed(content, "rs");

        assert!(!result.was_summarized);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_summarizer_rust_keeps_signatures() {
        let summarizer = FileSummarizer::new(10, 100); // Low threshold to trigger summarization

        let content = r#"use std::collections::HashMap;

pub struct Config {
    name: String,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}
"#;

        let result = summarizer.summarize_if_needed(content, "rs");

        assert!(result.was_summarized);
        assert!(result.content.contains("use std::collections::HashMap"));
        assert!(result.content.contains("pub struct Config"));
        assert!(result.content.contains("impl Config"));
        assert!(result.content.contains("pub fn new"));
        assert!(result.content.contains("pub fn get_name"));
        assert!(result.content.contains("SUMMARIZED"));
    }

    #[test]
    fn test_summarizer_python_keeps_structure() {
        let summarizer = FileSummarizer::new(10, 100);

        let content = r#"import os
from typing import Dict

class Config:
    def __init__(self, name: str):
        self.name = name

    def get_name(self) -> str:
        return self.name
"#;

        let result = summarizer.summarize_if_needed(content, "py");

        assert!(result.was_summarized);
        assert!(result.content.contains("import os"));
        assert!(result.content.contains("from typing import Dict"));
        assert!(result.content.contains("class Config"));
        assert!(result.content.contains("def __init__"));
        assert!(result.content.contains("def get_name"));
    }

    #[test]
    fn test_summarizer_javascript_keeps_structure() {
        let summarizer = FileSummarizer::new(10, 100);

        let content = r#"import { useState } from 'react';

export function MyComponent() {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
}

class Helper {
    constructor() {}
}
"#;

        let result = summarizer.summarize_if_needed(content, "js");

        assert!(result.was_summarized);
        assert!(result.content.contains("import { useState }"));
        assert!(result.content.contains("export function MyComponent"));
        assert!(result.content.contains("class Helper"));
    }

    #[test]
    fn test_summarizer_generic_truncates() {
        let summarizer = FileSummarizer::new(10, 100);

        // Create content with many lines
        let content: String = (0..100)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let result = summarizer.summarize_if_needed(&content, "txt");

        assert!(result.was_summarized);
        assert!(result.content.contains("SUMMARIZED"));
        assert!(result.content.contains("lines omitted"));
    }

    #[test]
    fn test_summary_result_reduction_ratio() {
        let result = SummaryResult {
            content: "short".to_string(),
            was_summarized: true,
            original_tokens: 100,
            summary_tokens: 25,
        };

        assert!((result.reduction_ratio() - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_summary_result_reduction_ratio_zero_original() {
        let result = SummaryResult {
            content: "".to_string(),
            was_summarized: false,
            original_tokens: 0,
            summary_tokens: 0,
        };

        assert!((result.reduction_ratio() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_accessors() {
        let s = FileSummarizer::new(100, 50);
        assert_eq!(s.threshold_tokens(), 100);
        assert_eq!(s.target_tokens(), 50);
    }

    #[test]
    fn test_rust_keeps_mod_and_trait() {
        let summarizer = FileSummarizer::new(10, 500);
        let content = "mod foo;\npub trait Bar {\n    fn baz();\n}\n";
        let result = summarizer.summarize_if_needed(content, "rs");
        assert!(result.was_summarized);
        assert!(result.content.contains("mod foo"));
        assert!(result.content.contains("pub trait Bar"));
    }

    #[test]
    fn test_rust_enum_and_use() {
        let summarizer = FileSummarizer::new(10, 500);
        let content = "use crate::types;\npub enum Color {\n    Red,\n    Blue,\n}\n";
        let result = summarizer.summarize_if_needed(content, "rs");
        assert!(result.content.contains("use crate::types"));
        assert!(result.content.contains("pub enum Color"));
    }

    #[test]
    fn test_rust_async_fn_in_impl() {
        let summarizer = FileSummarizer::new(10, 500);
        let content = "impl Server {\n    pub async fn start(&self) {\n        // body\n    }\n}\n";
        let result = summarizer.summarize_if_needed(content, "rs");
        assert!(result.content.contains("pub async fn start"));
    }

    #[test]
    fn test_rust_private_struct_and_enum() {
        let summarizer = FileSummarizer::new(10, 500);
        let content = "struct Foo {\n    x: i32,\n}\nenum Bar {\n    A,\n}\n";
        let result = summarizer.summarize_if_needed(content, "rs");
        assert!(result.content.contains("struct Foo"));
        assert!(result.content.contains("enum Bar"));
    }

    #[test]
    fn test_python_decorators_and_async_def() {
        let summarizer = FileSummarizer::new(10, 500);
        let content = "@app.route('/')\nasync def handler():\n    pass\n";
        let result = summarizer.summarize_if_needed(content, "py");
        assert!(result.content.contains("@app.route"));
        assert!(result.content.contains("async def handler"));
    }

    #[test]
    fn test_javascript_require_and_class() {
        let summarizer = FileSummarizer::new(10, 500);
        let content = "const fs = require('fs');\nclass MyClass {\n    constructor() {}\n}\n";
        let result = summarizer.summarize_if_needed(content, "js");
        assert!(result.content.contains("const fs = require"));
        assert!(result.content.contains("class MyClass"));
    }

    #[test]
    fn test_javascript_async_function() {
        let summarizer = FileSummarizer::new(10, 500);
        let content = "async function fetchData() {\n    return null;\n}\n";
        let result = summarizer.summarize_if_needed(content, "js");
        assert!(result.content.contains("async function fetchData"));
    }

    #[test]
    fn test_javascript_export_with_brace() {
        let summarizer = FileSummarizer::new(10, 500);
        let content = "export function foo() {\n    return 1;\n}\nexport const bar = 42;\n";
        let result = summarizer.summarize_if_needed(content, "js");
        assert!(result.content.contains("export function foo"));
        assert!(result.content.contains("export const bar"));
    }

    #[test]
    fn test_typescript_uses_js_summarizer() {
        let summarizer = FileSummarizer::new(10, 500);
        let content = "import { x } from 'y';\nexport class Z {}\n";
        let result = summarizer.summarize_if_needed(content, "ts");
        assert!(result.content.contains("import { x }"));
        assert!(result.content.contains("export class Z"));
    }

    #[test]
    fn test_generic_short_content_returned_as_is() {
        let summarizer = FileSummarizer::new(10, 500);
        // 30 lines < 50, so returned as-is
        let content: String = (0..30)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let result = summarizer.summarize_if_needed(&content, "txt");
        assert!(result.was_summarized);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_extract_fn_signature_with_semicolon() {
        let summarizer = FileSummarizer::new(100, 50);
        let sig = summarizer.extract_fn_signature("    fn foo();");
        assert_eq!(sig, Some("    fn foo();".to_string()));
    }

    #[test]
    fn test_extract_fn_signature_no_brace_no_semi() {
        let summarizer = FileSummarizer::new(100, 50);
        let sig = summarizer.extract_fn_signature("    fn foo()");
        assert_eq!(sig, Some("    fn foo()".to_string()));
    }

    #[test]
    fn test_extract_js_export_no_brace() {
        let summarizer = FileSummarizer::new(100, 50);
        let sig = summarizer.extract_js_export_signature("export const x = 5;");
        assert_eq!(sig, None);
    }

    #[test]
    fn test_truncate_with_notice_short_content() {
        let summarizer = FileSummarizer::new(100, 50);
        let result = summarizer.truncate_with_notice("short", 1000);
        assert_eq!(result, "short");
    }

    #[test]
    fn test_truncate_with_notice_long_content() {
        let summarizer = FileSummarizer::new(100, 1);
        let long = "a".repeat(100);
        let result = summarizer.truncate_with_notice(&long, 1);
        assert!(result.contains("truncated"));
    }

    #[test]
    fn test_rust_summarize_triggers_truncation() {
        // Very low target so the summary itself is too long
        let summarizer = FileSummarizer::new(5, 1);
        let content: String = (0..200)
            .map(|i| format!("use crate::module{};\n", i))
            .collect();
        let result = summarizer.summarize_if_needed(&content, "rs");
        assert!(result.was_summarized);
        assert!(result.content.contains("truncated"));
    }

    #[test]
    fn test_summary_result_fields() {
        let r = SummaryResult {
            content: "x".to_string(),
            was_summarized: true,
            original_tokens: 200,
            summary_tokens: 50,
        };
        assert_eq!(r.reduction_ratio(), 0.25);
        assert!(r.was_summarized);
    }
}
