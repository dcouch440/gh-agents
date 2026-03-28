//! Classify file-level changes across parallel step overlays.
//!
//! Walks each step's OverlayFS diff and categorizes every file path
//! into one of the `FileClassification` variants.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use super::types::{
    FileClassification, FileType, Language, MarkupKind, OverlayChange, StepOverlay, StructuredKind,
};

/// Binary file extensions — no line-merge possible.
const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "bmp", "webp", "svg", "woff", "woff2", "ttf", "otf", "eot",
    "pdf", "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "wasm", "so", "dll", "dylib", "exe", "o",
    "a", "lib", "pyc", "pyo", "class", "jar", "war", "mp3", "mp4", "avi", "mov", "wav", "ogg",
    "flac", "sqlite", "db",
];

/// Classify all file paths across the overlay diffs.
///
/// Returns a map from workspace-relative path to its classification.
/// `base_files` is the set of files that existed in the JuiceFS base
/// before the parallel batch started.
pub fn classify_overlays(
    overlays: &[StepOverlay],
    base_files: &HashMap<PathBuf, Vec<u8>>,
) -> HashMap<PathBuf, FileClassification> {
    // Group all changes by path
    let mut by_path: HashMap<PathBuf, Vec<(Uuid, &OverlayChange)>> = HashMap::new();
    for overlay in overlays {
        for (path, change) in &overlay.diff {
            by_path
                .entry(path.clone())
                .or_default()
                .push((overlay.step_id, change));
        }
    }

    let mut result = HashMap::new();

    for (path, changes) in by_path {
        let existed_in_base = base_files.contains_key(&path);
        let is_binary = detect_binary(&path, &changes);

        let classification = if is_binary {
            classify_binary(&changes)
        } else if existed_in_base {
            classify_existing_file(&changes)
        } else {
            classify_new_file(&changes)
        };

        result.insert(path, classification);
    }

    result
}

/// Detect the file type from its extension.
pub fn detect_file_type(path: &Path) -> FileType {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Check binary first
    if BINARY_EXTENSIONS.contains(&ext.as_str()) {
        return FileType::Binary;
    }

    // Config files by name — check BEFORE extension so Cargo.toml is Config, not Structured(Toml)
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    match file_name {
        "Cargo.toml"
        | "package.json"
        | "package-lock.json"
        | "requirements.txt"
        | "Pipfile"
        | "Gemfile"
        | "Dockerfile"
        | "docker-compose.yml"
        | "docker-compose.yaml"
        | ".env"
        | ".gitignore"
        | "Makefile"
        | "tsconfig.json"
        | "pyproject.toml"
        | "setup.py"
        | "setup.cfg" => {
            return FileType::Config;
        }
        _ => {}
    }

    // Code files
    match ext.as_str() {
        "py" => return FileType::Code(Language::Python),
        "js" | "jsx" | "mjs" | "cjs" => return FileType::Code(Language::JavaScript),
        "ts" | "tsx" => return FileType::Code(Language::TypeScript),
        "rs" => return FileType::Code(Language::Rust),
        "go" => return FileType::Code(Language::Go),
        "java" => return FileType::Code(Language::Java),
        "rb" => return FileType::Code(Language::Ruby),
        _ => {}
    }

    // Markup
    match ext.as_str() {
        "md" | "markdown" => return FileType::Markup(MarkupKind::Markdown),
        "rst" => return FileType::Markup(MarkupKind::ReStructuredText),
        "txt" => return FileType::Markup(MarkupKind::PlainText),
        _ => {}
    }

    // Structured data
    match ext.as_str() {
        "json" => return FileType::Structured(StructuredKind::Json),
        "yaml" | "yml" => return FileType::Structured(StructuredKind::Yaml),
        "toml" => return FileType::Structured(StructuredKind::Toml),
        "xml" => return FileType::Structured(StructuredKind::Xml),
        _ => {}
    }

    // Fallback: check some common code extensions we might have missed
    match ext.as_str() {
        "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" | "cs" | "swift" | "kt" | "scala" | "php"
        | "pl" | "pm" | "sh" | "bash" | "zsh" | "fish" | "lua" | "r" | "jl" | "ex" | "exs"
        | "erl" | "hrl" | "zig" | "nim" | "v" | "d" => {
            return FileType::Code(Language::Other(ext));
        }
        _ => {}
    }

    FileType::Unknown
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn detect_binary(path: &Path, changes: &[(Uuid, &OverlayChange)]) -> bool {
    // Check extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if BINARY_EXTENSIONS.contains(&ext.as_str()) {
        return true;
    }

    // Check content for null bytes in first 8KB
    for (_step_id, change) in changes {
        let content = match change {
            OverlayChange::Created(c) | OverlayChange::Modified(c) => c,
            OverlayChange::Deleted => continue,
        };
        let check_len = content.len().min(8192);
        if content[..check_len].contains(&0) {
            return true;
        }
    }

    false
}

fn classify_binary(changes: &[(Uuid, &OverlayChange)]) -> FileClassification {
    let versions: Vec<_> = changes
        .iter()
        .filter_map(|(id, change)| match change {
            OverlayChange::Created(c) | OverlayChange::Modified(c) => Some((*id, c.clone())),
            OverlayChange::Deleted => None,
        })
        .collect();

    if versions.len() <= 1 {
        match versions.into_iter().next() {
            Some((_id, content)) => FileClassification::BinarySingle { content },
            None => FileClassification::DeletedSingle,
        }
    } else {
        FileClassification::BinaryMulti { versions }
    }
}

fn classify_existing_file(changes: &[(Uuid, &OverlayChange)]) -> FileClassification {
    let mut modifiers = Vec::new();
    let mut deleters = Vec::new();

    for (step_id, change) in changes {
        match change {
            OverlayChange::Modified(content) => modifiers.push((*step_id, content.clone())),
            OverlayChange::Deleted => deleters.push(*step_id),
            OverlayChange::Created(content) => modifiers.push((*step_id, content.clone())),
        }
    }

    // Delete-modify conflict
    if !deleters.is_empty() && !modifiers.is_empty() {
        let (id, content) = modifiers.into_iter().next().unwrap();
        return FileClassification::DeletedConflict {
            modifier_step_id: id,
            modified_content: content,
        };
    }

    // Pure deletion
    if !deleters.is_empty() {
        return FileClassification::DeletedSingle;
    }

    // Single modifier
    if modifiers.len() == 1 {
        let (_id, content) = modifiers.into_iter().next().unwrap();
        return FileClassification::ModifiedSingle { content };
    }

    // Multi-modifier — needs diff3
    FileClassification::ModifiedMulti {
        versions: modifiers,
    }
}

fn classify_new_file(changes: &[(Uuid, &OverlayChange)]) -> FileClassification {
    let creators: Vec<_> = changes
        .iter()
        .filter_map(|(id, change)| match change {
            OverlayChange::Created(c) => Some((*id, c.clone())),
            _ => None,
        })
        .collect();

    if creators.len() == 1 {
        let (_id, content) = creators.into_iter().next().unwrap();
        FileClassification::NewFileSingle { content }
    } else {
        FileClassification::NewFileMulti { versions: creators }
    }
}
