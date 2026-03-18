//! Denylist filter for overlay diffs (B4).
//!
//! Strips build artifacts, caches, and other junk from overlay diffs
//! before persisting clean files to JuiceFS. Uses `.gitignore`-style
//! pattern matching via the `ignore` crate.

use std::path::Path;

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use once_cell::sync::Lazy;

use crate::constants::OVERLAY_DENYLIST_PATTERNS;

use super::types::StepOverlay;

/// Compiled denylist matcher, built once from `OVERLAY_DENYLIST_PATTERNS`.
static DENYLIST: Lazy<Gitignore> = Lazy::new(|| {
    let mut builder = GitignoreBuilder::new("");
    for pattern in OVERLAY_DENYLIST_PATTERNS {
        builder.add_line(None, pattern).ok();
    }
    builder
        .build()
        .unwrap_or_else(|_| GitignoreBuilder::new("").build().unwrap())
});

/// Returns `true` if the path matches any denylist pattern.
///
/// Uses full `.gitignore` semantics: `node_modules/` matches any file
/// under a `node_modules` directory, `*.pyc` matches file extensions.
pub(crate) fn is_denylisted(path: &Path) -> bool {
    DENYLIST
        .matched_path_or_any_parents(path, false)
        .is_ignore()
}

/// Remove denylisted entries from a `StepOverlay` in-place.
///
/// Returns the count of entries removed.
pub(crate) fn filter_overlay(overlay: &mut StepOverlay) -> usize {
    let before = overlay.diff.len();
    overlay.diff.retain(|path, _| !is_denylisted(path));
    before - overlay.diff.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn git_dir_denied() {
        assert!(is_denylisted(Path::new(".git/HEAD")));
        assert!(is_denylisted(Path::new(".git/objects/pack/abc123")));
    }

    #[test]
    fn node_modules_denied() {
        assert!(is_denylisted(Path::new("node_modules/express/index.js")));
        assert!(is_denylisted(Path::new(
            "frontend/node_modules/react/index.js"
        )));
    }

    #[test]
    fn python_cache_denied() {
        assert!(is_denylisted(Path::new(
            "__pycache__/module.cpython-311.pyc"
        )));
        assert!(is_denylisted(Path::new("src/__pycache__/foo.pyc")));
    }

    #[test]
    fn pyc_extension_denied() {
        assert!(is_denylisted(Path::new("foo.pyc")));
        assert!(is_denylisted(Path::new("src/bar.pyc")));
    }

    #[test]
    fn so_extension_denied() {
        assert!(is_denylisted(Path::new("lib/libfoo.so")));
    }

    #[test]
    fn target_dir_denied() {
        assert!(is_denylisted(Path::new("target/debug/binary")));
        assert!(is_denylisted(Path::new("my_app/target/release/lib.so")));
    }

    #[test]
    fn venv_denied() {
        assert!(is_denylisted(Path::new(".venv/lib/python3.11/site.py")));
        assert!(is_denylisted(Path::new("venv/bin/activate")));
    }

    #[test]
    fn clean_files_allowed() {
        assert!(!is_denylisted(Path::new("src/main.py")));
        assert!(!is_denylisted(Path::new("results/output.txt")));
        assert!(!is_denylisted(Path::new("README.md")));
        assert!(!is_denylisted(Path::new("my_app/main.rs")));
    }

    #[test]
    fn build_tools_not_denied() {
        // "build/" pattern should match the directory "build", not a prefix "build-"
        assert!(!is_denylisted(Path::new("build-tools/run.sh")));
    }

    #[test]
    fn filter_overlay_removes_junk() {
        use crate::server::hub::dag::merge::types::{OverlayChange, OverlayDiff};

        let mut diff = OverlayDiff::new();
        diff.insert(
            PathBuf::from("src/main.py"),
            OverlayChange::Created(b"clean".to_vec()),
        );
        diff.insert(
            PathBuf::from("__pycache__/main.cpython-311.pyc"),
            OverlayChange::Created(b"junk".to_vec()),
        );
        diff.insert(
            PathBuf::from("node_modules/pkg/index.js"),
            OverlayChange::Created(b"junk".to_vec()),
        );
        diff.insert(
            PathBuf::from("results/data.json"),
            OverlayChange::Created(b"clean".to_vec()),
        );

        let mut overlay = StepOverlay {
            step_id: uuid::Uuid::new_v4(),
            step_name: "test".to_string(),
            step_description: "test".to_string(),
            display_order: 0,
            diff,
        };

        let removed = filter_overlay(&mut overlay);
        assert_eq!(removed, 2);
        assert_eq!(overlay.diff.len(), 2);
        assert!(overlay.diff.contains_key(&PathBuf::from("src/main.py")));
        assert!(overlay
            .diff
            .contains_key(&PathBuf::from("results/data.json")));
    }
}
