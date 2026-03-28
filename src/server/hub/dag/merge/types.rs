//! Types for the parallel workspace merge system.

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

use uuid::Uuid;

// ── File-Level Classification ────────────────────────────────────────────────

/// A change detected in a single step's OverlayFS upper directory.
#[derive(Debug, Clone)]
pub enum OverlayChange {
    /// File was created (did not exist in base).
    Created(Vec<u8>),
    /// File was modified (existed in base, content differs).
    Modified(Vec<u8>),
    /// File was deleted (existed in base, removed in overlay).
    Deleted,
}

/// Per-step overlay diff: maps workspace-relative paths to changes.
pub type OverlayDiff = HashMap<PathBuf, OverlayChange>;

/// A step's overlay diff with its step metadata.
#[derive(Debug)]
pub struct StepOverlay {
    pub step_id: Uuid,
    pub step_name: String,
    pub step_description: String,
    pub display_order: i32,
    pub diff: OverlayDiff,
}

/// Classification of a file path across all parallel step overlays.
#[derive(Debug)]
pub enum FileClassification {
    /// Created by exactly one step — auto-accept.
    NewFileSingle { content: Vec<u8> },
    /// Same path created by 2+ steps — LLM merge needed.
    NewFileMulti { versions: Vec<(Uuid, Vec<u8>)> },
    /// Modified by exactly one step — auto-accept.
    ModifiedSingle { content: Vec<u8> },
    /// Modified by 2+ steps — needs three-way merge.
    ModifiedMulti { versions: Vec<(Uuid, Vec<u8>)> },
    /// Deleted by one step, untouched by others — auto-accept deletion.
    DeletedSingle,
    /// Deleted by one step, modified by another — keep modified + warn.
    DeletedConflict {
        modifier_step_id: Uuid,
        modified_content: Vec<u8>,
    },
    /// Binary file from one step — auto-accept.
    BinarySingle { content: Vec<u8> },
    /// Binary file from 2+ steps — last-write-wins.
    BinaryMulti { versions: Vec<(Uuid, Vec<u8>)> },
}

// ── File Type Detection ──────────────────────────────────────────────────────

/// Detected file type, used for context extraction strategy.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FileType {
    Code(Language),
    Markup(MarkupKind),
    Structured(StructuredKind),
    Config,
    Binary,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Go,
    Java,
    Ruby,
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarkupKind {
    Markdown,
    ReStructuredText,
    PlainText,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructuredKind {
    Json,
    Yaml,
    Toml,
    Xml,
}

// ── Conflict Types ───────────────────────────────────────────────────────────

/// A single conflict region extracted from diff3 conflict markers.
#[derive(Debug, Clone)]
pub struct ConflictHunk {
    /// Base version of the conflicting lines.
    pub base_lines: String,
    /// Version A's modification.
    pub version_a_lines: String,
    /// Version B's modification.
    pub version_b_lines: String,
    /// Line range in the original base file (approximate).
    pub base_line_range: Range<usize>,
}

/// Context extracted for a conflict hunk, tailored to file type.
#[derive(Debug, Clone, Default)]
pub struct ConflictContext {
    /// File path relative to workspace root.
    pub file_path: String,
    /// Detected file type.
    pub file_type: FileType,
    /// Import block at top of file (code files only).
    pub import_block: Option<String>,
    /// Document heading outline (markdown only).
    pub document_outline: Option<String>,
    /// Enclosing function/class/section scope.
    pub enclosing_scope: Option<ScopeInfo>,
    /// Lines surrounding the conflict.
    pub surrounding_lines: String,
    /// Full file content (for small configs/structured data).
    pub full_file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScopeInfo {
    /// The scope type (function, class, impl, section heading).
    pub kind: String,
    /// The scope name (function name, class name, heading text).
    pub name: String,
    /// The full text of the enclosing scope.
    pub content: String,
    /// Start line of the scope in the file.
    pub start_line: usize,
}

// ── Merge Results ────────────────────────────────────────────────────────────

/// Result of a three-way merge on a single file.
#[derive(Debug)]
pub enum MergeResult {
    /// Clean merge — no conflicts.
    Clean(String),
    /// Merge produced conflicts that need LLM resolution.
    Conflicts {
        /// The conflicted output with markers.
        conflicted: String,
        /// Parsed conflict hunks.
        hunks: Vec<ConflictHunk>,
    },
}

/// Summary of merge operations for a parallel batch.
#[derive(Debug, Default)]
pub struct MergeReport {
    /// Files auto-accepted (single step or clean diff3).
    pub auto_merged: usize,
    /// Files that required LLM resolution.
    pub llm_resolved: usize,
    /// Total conflict hunks sent to LLM.
    pub conflict_hunks: usize,
    /// Files that used fallback (binary, too large, LLM failure).
    pub fallback_used: usize,
    /// Total LLM tokens used for merge resolution.
    pub total_tokens: u64,
}

/// Info about a step used in merge prompts.
#[derive(Debug, Clone)]
pub struct StepInfo {
    pub name: String,
    pub description: String,
    pub display_order: i32,
}
