#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::execution::diagnostics::types::ChangeType;
    use crate::execution::diagnostics::workspace::snapshot::{parse_snapshot, FileMetadata};
    use crate::execution::diagnostics::workspace::WorkspaceTracker;

    #[test]
    fn parse_snapshot_basic() {
        let output = "main.py\tf\t420\t1710000000.0\nsrc\td\t4096\t1710000001.0\n";
        let snap = parse_snapshot(output);
        assert_eq!(snap.entries.len(), 2);
        assert_eq!(snap.entries[&PathBuf::from("main.py")].file_type, 'f');
        assert_eq!(snap.entries[&PathBuf::from("main.py")].size, 420);
        assert_eq!(snap.entries[&PathBuf::from("src")].file_type, 'd');
    }

    #[test]
    fn parse_snapshot_empty() {
        let snap = parse_snapshot("");
        assert!(snap.entries.is_empty());
    }

    #[test]
    fn parse_snapshot_malformed_lines_skipped() {
        let output = "good.py\tf\t100\t1710000000.0\nbad line\n\nanother_bad\t\n";
        let snap = parse_snapshot(output);
        assert_eq!(snap.entries.len(), 1);
    }

    #[test]
    fn file_count_and_dir_count() {
        let output = "a.py\tf\t100\t1.0\nb.py\tf\t200\t2.0\nsrc\td\t4096\t3.0\n";
        let snap = parse_snapshot(output);
        assert_eq!(snap.file_count(), 2);
        assert_eq!(snap.dir_count(), 1);
        assert_eq!(snap.total_size(), 300);
    }

    #[test]
    fn diff_created_file() {
        let before = parse_snapshot("a.py\tf\t100\t1.0\n");
        let after = parse_snapshot("a.py\tf\t100\t1.0\nb.py\tf\t200\t2.0\n");
        let changes = WorkspaceTracker::diff(&before, &after);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("b.py"));
        assert_eq!(changes[0].change_type, ChangeType::Created);
        assert_eq!(changes[0].size, 200);
    }

    #[test]
    fn diff_modified_file_size_change() {
        let before = parse_snapshot("a.py\tf\t100\t1.0\n");
        let after = parse_snapshot("a.py\tf\t150\t2.0\n");
        let changes = WorkspaceTracker::diff(&before, &after);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Modified);
        assert_eq!(changes[0].size, 150);
    }

    #[test]
    fn diff_modified_file_mtime_change() {
        let before = parse_snapshot("a.py\tf\t100\t1.0\n");
        let after = parse_snapshot("a.py\tf\t100\t5.0\n");
        let changes = WorkspaceTracker::diff(&before, &after);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Modified);
    }

    #[test]
    fn diff_unchanged_file() {
        let before = parse_snapshot("a.py\tf\t100\t1.0\n");
        let after = parse_snapshot("a.py\tf\t100\t1.0\n");
        let changes = WorkspaceTracker::diff(&before, &after);
        assert!(changes.is_empty());
    }

    #[test]
    fn diff_deleted_file_missing_from_after() {
        let before = parse_snapshot("a.py\tf\t100\t1.0\nb.py\tf\t200\t2.0\n");
        let after = parse_snapshot("a.py\tf\t100\t1.0\n");
        let changes = WorkspaceTracker::diff(&before, &after);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("b.py"));
        assert_eq!(changes[0].change_type, ChangeType::Deleted);
    }

    #[test]
    fn diff_whiteout_file() {
        let before = parse_snapshot("a.py\tf\t100\t1.0\n");
        let after = parse_snapshot("a.py\tf\t100\t1.0\n.wh.old.py\tc\t0\t3.0\n");
        let changes = WorkspaceTracker::diff(&before, &after);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("old.py"));
        assert_eq!(changes[0].change_type, ChangeType::Deleted);
    }

    #[test]
    fn diff_whiteout_in_subdirectory() {
        let before = parse_snapshot("");
        let after = parse_snapshot("src/.wh.removed.py\tc\t0\t1.0\n");
        let changes = WorkspaceTracker::diff(&before, &after);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("src/removed.py"));
        assert_eq!(changes[0].change_type, ChangeType::Deleted);
    }

    #[test]
    fn diff_directories_not_reported() {
        let before = parse_snapshot("");
        let after = parse_snapshot("src\td\t4096\t1.0\nsrc/main.py\tf\t100\t1.0\n");
        let changes = WorkspaceTracker::diff(&before, &after);
        // Only the file, not the directory
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("src/main.py"));
    }

    #[test]
    fn diff_mixed_changes() {
        let before = parse_snapshot("a.py\tf\t100\t1.0\nb.py\tf\t200\t2.0\nc.py\tf\t300\t3.0\n");
        let after = parse_snapshot("a.py\tf\t100\t1.0\nb.py\tf\t250\t4.0\nd.py\tf\t400\t5.0\n");
        let changes = WorkspaceTracker::diff(&before, &after);
        assert_eq!(changes.len(), 3);
        // Sorted by path: b.py Modified, c.py Deleted, d.py Created
        assert_eq!(changes[0].path, PathBuf::from("b.py"));
        assert_eq!(changes[0].change_type, ChangeType::Modified);
        assert_eq!(changes[1].path, PathBuf::from("c.py"));
        assert_eq!(changes[1].change_type, ChangeType::Deleted);
        assert_eq!(changes[2].path, PathBuf::from("d.py"));
        assert_eq!(changes[2].change_type, ChangeType::Created);
    }

    #[test]
    fn digest_with_changes() {
        let mut tracker = WorkspaceTracker::new();
        let before = parse_snapshot("a.py\tf\t100\t1.0\n");
        tracker.update(&before);

        let after = parse_snapshot("a.py\tf\t100\t1.0\nb.py\tf\t200\t2.0\nc.py\tf\t300\t3.0\n");
        let digest = tracker.digest(&after);
        assert_eq!(digest.file_count, 3);
        assert_eq!(digest.file_delta, 2);
        assert_eq!(digest.total_size, 600);
        assert_eq!(digest.last_modified, Some(PathBuf::from("c.py")));
    }

    #[test]
    fn digest_first_command_with_initialize() {
        let mut tracker = WorkspaceTracker::new();
        // Simulate: workspace already has 10 files from prior steps
        let before = parse_snapshot("a.py\tf\t100\t1.0\nb.py\tf\t200\t2.0\nc.py\tf\t300\t3.0\n");
        tracker.initialize(&before);

        // After command, one new file created
        let after = parse_snapshot(
            "a.py\tf\t100\t1.0\nb.py\tf\t200\t2.0\nc.py\tf\t300\t3.0\nd.py\tf\t400\t4.0\n",
        );
        let digest = tracker.digest(&after);
        assert_eq!(digest.file_count, 4);
        assert_eq!(digest.file_delta, 1); // 4 - 3 = 1, not 4 - 0 = 4
    }

    #[test]
    fn digest_first_command_no_changes() {
        let mut tracker = WorkspaceTracker::new();
        let snapshot = parse_snapshot("a.py\tf\t100\t1.0\n");
        tracker.initialize(&snapshot);
        let digest = tracker.digest(&snapshot);
        assert_eq!(digest.file_delta, 0); // no change
    }

    #[test]
    fn digest_render_format() {
        let mut tracker = WorkspaceTracker::new();
        let before = parse_snapshot("a.py\tf\t100\t1.0\n");
        tracker.update(&before);

        let after = parse_snapshot("a.py\tf\t100\t1.0\nb.py\tf\t200\t2.0\nsrc\td\t4096\t3.0\n");
        let digest = tracker.digest(&after);
        let rendered = digest.render();
        assert!(rendered.contains("2 files (+1)"));
        assert!(rendered.contains("1 dirs"));
    }
}
