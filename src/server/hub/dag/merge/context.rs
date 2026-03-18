//! File-type-aware context extraction for conflict hunks.
//!
//! Extracts imports, enclosing scopes, document outlines, and surrounding
//! lines to give the LLM enough context for intelligent merge resolution.

use regex::Regex;

use crate::constants;

use super::types::{ConflictContext, ConflictHunk, FileType, Language, MarkupKind, ScopeInfo};

/// Extract context for a conflict hunk based on file type.
///
/// For code files, `version_a` and `version_b` are used to extract imports
/// from all three versions (base + both agents), giving the LLM a complete
/// picture of what each agent added.
pub fn extract_context(
    base_content: &str,
    hunk: &ConflictHunk,
    file_type: &FileType,
    file_path: &str,
    version_a: &str,
    version_b: &str,
) -> ConflictContext {
    match file_type {
        FileType::Code(lang) => {
            extract_code_context(base_content, hunk, lang, file_path, version_a, version_b)
        }
        FileType::Markup(kind) => extract_markup_context(base_content, hunk, kind, file_path),
        FileType::Structured(_) | FileType::Config => {
            extract_structured_context(base_content, hunk, file_type, file_path)
        }
        FileType::Binary => ConflictContext {
            file_path: file_path.to_string(),
            file_type: file_type.clone(),
            ..Default::default()
        },
        FileType::Unknown => extract_generic_context(base_content, hunk, file_path),
    }
}

// ── Code Context ─────────────────────────────────────────────────────────────

fn extract_code_context(
    content: &str,
    hunk: &ConflictHunk,
    lang: &Language,
    file_path: &str,
    version_a: &str,
    version_b: &str,
) -> ConflictContext {
    let lines: Vec<&str> = content.lines().collect();
    let conflict_start = hunk.base_line_range.start;
    let conflict_end = hunk.base_line_range.end;

    // 1. Always include imports — union from all three versions (base, A, B)
    //    so the LLM sees imports each agent added, not just the base's.
    let import_block = merge_import_blocks(content, version_a, version_b, lang, conflict_start);

    // 2. Find enclosing scope
    let enclosing_scope = find_enclosing_scope(&lines, conflict_start, lang);

    // 3. Surrounding context
    let ctx_lines = constants::MERGE_CONTEXT_LINES;
    let ctx_start = conflict_start.saturating_sub(ctx_lines);
    let ctx_end = (conflict_end + ctx_lines).min(lines.len());
    let surrounding = lines[ctx_start..ctx_end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:>4} {}", ctx_start + i + 1, line))
        .collect::<Vec<_>>()
        .join("\n");

    ConflictContext {
        file_path: file_path.to_string(),
        file_type: FileType::Code(lang.clone()),
        import_block,
        enclosing_scope,
        surrounding_lines: surrounding,
        ..Default::default()
    }
}

/// Extract import lines from all three versions, deduplicate, and return as a
/// single block. This ensures the LLM sees imports each agent added, not just
/// what was in the base.
fn merge_import_blocks(
    base: &str,
    version_a: &str,
    version_b: &str,
    lang: &Language,
    conflict_start: usize,
) -> Option<String> {
    let base_lines: Vec<&str> = base.lines().collect();
    let a_lines: Vec<&str> = version_a.lines().collect();
    let b_lines: Vec<&str> = version_b.lines().collect();

    let base_end = find_import_block_end(&base_lines, lang);
    let a_end = find_import_block_end(&a_lines, lang);
    let b_end = find_import_block_end(&b_lines, lang);

    // If no version has imports, skip
    if base_end == 0 && a_end == 0 && b_end == 0 {
        return None;
    }

    // If the conflict is within the base's import block, skip — the hunk itself has the imports
    if conflict_start < base_end {
        return None;
    }

    // Collect all import lines, deduplicate by trimmed content
    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::new();

    for source in [
        &base_lines[..base_end],
        &a_lines[..a_end],
        &b_lines[..b_end],
    ] {
        for line in source {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if seen.insert(trimmed.to_string()) {
                merged.push(*line);
            }
        }
    }

    if merged.is_empty() {
        None
    } else {
        Some(merged.join("\n"))
    }
}

/// Find where the import block ends (first non-import, non-blank, non-comment line).
fn find_import_block_end(lines: &[&str], lang: &Language) -> usize {
    let mut last_import_line = 0;
    let mut in_imports = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_imports {
                continue; // Blank lines within the import block are fine
            }
            continue;
        }

        let is_import = match lang {
            Language::Python => {
                trimmed.starts_with("import ")
                    || trimmed.starts_with("from ")
                    || trimmed.starts_with('#')
            }
            Language::JavaScript | Language::TypeScript => {
                trimmed.starts_with("import ")
                    || trimmed.starts_with("const ") && trimmed.contains("require(")
                    || trimmed.starts_with("export ")
                    || trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
            }
            Language::Rust => {
                trimmed.starts_with("use ")
                    || trimmed.starts_with("pub use ")
                    || trimmed.starts_with("mod ")
                    || trimmed.starts_with("pub mod ")
                    || trimmed.starts_with("extern crate ")
                    || trimmed.starts_with("//")
                    || trimmed.starts_with("#[")
                    || trimmed.starts_with("#!")
            }
            Language::Go => {
                trimmed.starts_with("import ")
                    || trimmed.starts_with("import (")
                    || trimmed == ")"
                    || trimmed.starts_with('"')
                    || trimmed.starts_with("//")
                    || trimmed.starts_with("package ")
            }
            _ => {
                trimmed.starts_with("import ")
                    || trimmed.starts_with("from ")
                    || trimmed.starts_with("require")
                    || trimmed.starts_with("//")
                    || trimmed.starts_with('#')
            }
        };

        if is_import {
            in_imports = true;
            last_import_line = i + 1;
        } else if in_imports {
            // First non-import line after imports — we're done
            break;
        } else if i > 30 {
            // If we haven't found any imports in 30 lines, stop looking
            break;
        }
    }

    last_import_line
}

/// Walk backwards from conflict line to find the enclosing scope.
fn find_enclosing_scope(
    lines: &[&str],
    conflict_line: usize,
    lang: &Language,
) -> Option<ScopeInfo> {
    let scope_pattern = match lang {
        Language::Python => r"^(\s*)(def |class |async def )",
        Language::JavaScript | Language::TypeScript => {
            r"^(\s*)(function |class |const \w+ = |export (default )?(function |class ))"
        }
        Language::Rust => r"^(\s*)(pub )?(fn |impl |mod |struct |enum |trait )",
        Language::Go => r"^(\s*)func ",
        Language::Java => {
            r"^(\s*)(public |private |protected )?(static )?(void |int |String |class )"
        }
        _ => r"^(\s*)(def |fn |func |function |class )",
    };

    let re = Regex::new(scope_pattern).ok()?;

    // Get indentation level at conflict line
    let conflict_indent = if conflict_line < lines.len() {
        lines[conflict_line].len() - lines[conflict_line].trim_start().len()
    } else {
        0
    };

    // Walk backwards to find a scope with less indentation
    for i in (0..conflict_line).rev() {
        if let Some(caps) = re.captures(lines[i]) {
            let scope_indent = caps.get(1).map_or(0, |m| m.as_str().len());
            if scope_indent < conflict_indent || conflict_indent == 0 {
                // Found enclosing scope — determine its end
                let scope_end = find_scope_end(lines, i, scope_indent, lang);
                let name = extract_scope_name(lines[i], lang);
                let content = lines[i..scope_end].join("\n");

                return Some(ScopeInfo {
                    kind: detect_scope_kind(lines[i], lang),
                    name,
                    content,
                    start_line: i,
                });
            }
        }
    }

    None
}

/// Find where a scope ends (next line with equal or less indentation, or file end).
fn find_scope_end(lines: &[&str], start: usize, scope_indent: usize, lang: &Language) -> usize {
    // For brace-based languages, count braces
    if matches!(
        lang,
        Language::Rust
            | Language::JavaScript
            | Language::TypeScript
            | Language::Go
            | Language::Java
    ) {
        let mut brace_depth = 0;
        let mut found_open = false;
        for i in start..lines.len() {
            for ch in lines[i].chars() {
                if ch == '{' {
                    brace_depth += 1;
                    found_open = true;
                } else if ch == '}' {
                    brace_depth -= 1;
                    if found_open && brace_depth == 0 {
                        return (i + 1).min(lines.len());
                    }
                }
            }
        }
    }

    // For indentation-based (Python) or fallback
    for i in (start + 1)..lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            continue;
        }
        let indent = lines[i].len() - trimmed.len();
        if indent <= scope_indent && !trimmed.starts_with('#') && !trimmed.starts_with("//") {
            return i;
        }
    }

    lines.len()
}

fn extract_scope_name(line: &str, _lang: &Language) -> String {
    let trimmed = line.trim();
    // Extract the name after def/fn/class/func/function keywords
    let keywords = [
        "async def ",
        "def ",
        "fn ",
        "func ",
        "function ",
        "class ",
        "impl ",
        "struct ",
        "enum ",
        "trait ",
        "pub fn ",
        "pub struct ",
        "pub enum ",
        "pub trait ",
        "pub(crate) fn ",
        "pub(crate) struct ",
    ];
    for kw in &keywords {
        if let Some(rest) = trimmed.strip_prefix(kw) {
            // Take until ( or { or : or <
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return name;
            }
        }
    }
    trimmed.chars().take(40).collect()
}

fn detect_scope_kind(line: &str, _lang: &Language) -> String {
    let trimmed = line.trim();
    if trimmed.contains("class ") {
        "class".to_string()
    } else if trimmed.contains("impl ") {
        "impl".to_string()
    } else if trimmed.contains("trait ") {
        "trait".to_string()
    } else if trimmed.contains("struct ") {
        "struct".to_string()
    } else if trimmed.contains("enum ") {
        "enum".to_string()
    } else if trimmed.contains("mod ") {
        "module".to_string()
    } else {
        "function".to_string()
    }
}

// ── Markup Context ───────────────────────────────────────────────────────────

fn extract_markup_context(
    content: &str,
    hunk: &ConflictHunk,
    kind: &MarkupKind,
    file_path: &str,
) -> ConflictContext {
    let lines: Vec<&str> = content.lines().collect();
    let conflict_start = hunk.base_line_range.start;
    let conflict_end = hunk.base_line_range.end;

    match kind {
        MarkupKind::Markdown => {
            let outline = extract_heading_outline(&lines);
            let section = find_enclosing_section(&lines, conflict_start);

            let ctx_lines = constants::MERGE_CONTEXT_LINES_MARKDOWN;
            let ctx_start = conflict_start.saturating_sub(ctx_lines);
            let ctx_end = (conflict_end + ctx_lines).min(lines.len());
            let surrounding = lines[ctx_start..ctx_end].join("\n");

            ConflictContext {
                file_path: file_path.to_string(),
                file_type: FileType::Markup(kind.clone()),
                document_outline: Some(outline),
                enclosing_scope: section,
                surrounding_lines: surrounding,
                ..Default::default()
            }
        }
        _ => extract_generic_context(content, hunk, file_path),
    }
}

/// Build a heading outline from markdown content.
fn extract_heading_outline(lines: &[&str]) -> String {
    let mut outline = Vec::new();
    let heading_re = Regex::new(r"^(#{1,6})\s+(.+)").unwrap();

    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = heading_re.captures(line) {
            let level = caps.get(1).unwrap().as_str().len();
            let text = caps.get(2).unwrap().as_str();
            let indent = "  ".repeat(level.saturating_sub(1));
            outline.push(format!("{}{} (line {})", indent, text, i + 1));
        }
    }

    outline.join("\n")
}

/// Find the enclosing markdown section (from heading to next heading of equal/higher level).
fn find_enclosing_section(lines: &[&str], target_line: usize) -> Option<ScopeInfo> {
    let heading_re = Regex::new(r"^(#{1,6})\s+(.+)").unwrap();

    // Walk backwards to find the nearest heading
    let mut section_start = 0;
    let mut section_level = 0;
    let mut section_name = String::new();

    for i in (0..=target_line.min(lines.len().saturating_sub(1))).rev() {
        if let Some(caps) = heading_re.captures(lines[i]) {
            section_start = i;
            section_level = caps.get(1).unwrap().as_str().len();
            section_name = caps.get(2).unwrap().as_str().to_string();
            break;
        }
    }

    // Walk forward to find the end (next heading of equal or higher level)
    let mut section_end = lines.len();
    for i in (section_start + 1)..lines.len() {
        if let Some(caps) = heading_re.captures(lines[i]) {
            let level = caps.get(1).unwrap().as_str().len();
            if level <= section_level {
                section_end = i;
                break;
            }
        }
    }

    // Cap section length at 200 lines
    let effective_end = section_end.min(section_start + 200);
    let content = lines[section_start..effective_end].join("\n");

    Some(ScopeInfo {
        kind: "section".to_string(),
        name: section_name,
        content,
        start_line: section_start,
    })
}

// ── Structured Data Context ──────────────────────────────────────────────────

fn extract_structured_context(
    content: &str,
    hunk: &ConflictHunk,
    file_type: &FileType,
    file_path: &str,
) -> ConflictContext {
    let lines: Vec<&str> = content.lines().collect();

    // For small files (<200 lines), include the entire file
    if lines.len() < 200 {
        return ConflictContext {
            file_path: file_path.to_string(),
            file_type: file_type.clone(),
            full_file: Some(content.to_string()),
            ..Default::default()
        };
    }

    // For larger files, extract surrounding context
    let conflict_start = hunk.base_line_range.start;
    let conflict_end = hunk.base_line_range.end;
    let ctx_start = conflict_start.saturating_sub(30);
    let ctx_end = (conflict_end + 30).min(lines.len());
    let surrounding = lines[ctx_start..ctx_end].join("\n");

    ConflictContext {
        file_path: file_path.to_string(),
        file_type: file_type.clone(),
        surrounding_lines: surrounding,
        ..Default::default()
    }
}

// ── Generic Context ──────────────────────────────────────────────────────────

fn extract_generic_context(content: &str, hunk: &ConflictHunk, file_path: &str) -> ConflictContext {
    let lines: Vec<&str> = content.lines().collect();
    let conflict_start = hunk.base_line_range.start;
    let conflict_end = hunk.base_line_range.end;

    let ctx_start = conflict_start.saturating_sub(constants::MERGE_CONTEXT_LINES);
    let ctx_end = (conflict_end + constants::MERGE_CONTEXT_LINES).min(lines.len());
    let surrounding = lines[ctx_start..ctx_end].join("\n");

    ConflictContext {
        file_path: file_path.to_string(),
        file_type: FileType::Unknown,
        surrounding_lines: surrounding,
        ..Default::default()
    }
}
