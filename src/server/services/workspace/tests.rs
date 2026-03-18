#[cfg(test)]
mod tests {
    use std::fs;

    use uuid::Uuid;

    use crate::server::services::workspace::WorkspaceManager;

    /// Create a temp dir and return a workspace manager rooted there.
    fn test_manager() -> (WorkspaceManager, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let mgr = WorkspaceManager::new(tmp.path()).expect("create manager");
        (mgr, tmp)
    }

    #[test]
    fn new_fails_on_missing_path() {
        let result = WorkspaceManager::new("/nonexistent/path/juicefs");
        assert!(result.is_err());
    }

    #[test]
    fn create_and_destroy_workspace() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();

        // Create
        let path = mgr.create_run_workspace(wf, run).expect("create");
        assert!(path.exists());
        assert!(mgr.workspace_exists(wf, run));

        // Destroy
        let removed = mgr.destroy_run_workspace(wf, run).expect("destroy");
        assert!(removed);
        assert!(!mgr.workspace_exists(wf, run));

        // Destroy again returns false
        let removed = mgr.destroy_run_workspace(wf, run).expect("destroy again");
        assert!(!removed);
    }

    #[test]
    fn run_workspace_path_structure() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();

        let path = mgr.run_workspace_path(wf, run);
        let expected = mgr
            .mount_point()
            .join("workflows")
            .join(wf.to_string())
            .join("runs")
            .join(run.to_string());
        assert_eq!(path, expected);
    }

    #[test]
    fn list_files_empty_workspace() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();

        mgr.create_run_workspace(wf, run).expect("create");
        let files = mgr.list_files(wf, run, None).expect("list");
        assert!(files.is_empty());
    }

    #[test]
    fn list_files_with_content() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();

        let root = mgr.create_run_workspace(wf, run).expect("create");

        // Write some files
        fs::write(root.join("readme.md"), "hello").unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.py"), "print('hi')").unwrap();

        let files = mgr.list_files(wf, run, None).expect("list");
        assert_eq!(files.len(), 2);
        assert!(files.contains(&std::path::PathBuf::from("readme.md")));
        assert!(files.contains(&std::path::PathBuf::from("src/main.py")));
    }

    #[test]
    fn list_files_with_prefix() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();

        let root = mgr.create_run_workspace(wf, run).expect("create");

        fs::write(root.join("top.txt"), "top").unwrap();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/a.txt"), "a").unwrap();
        fs::write(root.join("sub/b.txt"), "b").unwrap();

        let files = mgr.list_files(wf, run, Some("sub")).expect("list");
        assert_eq!(files.len(), 2);
        assert!(files.contains(&std::path::PathBuf::from("sub/a.txt")));
        assert!(files.contains(&std::path::PathBuf::from("sub/b.txt")));
    }

    #[test]
    fn list_files_nonexistent_prefix() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();

        mgr.create_run_workspace(wf, run).expect("create");
        let files = mgr.list_files(wf, run, Some("nope")).expect("list");
        assert!(files.is_empty());
    }

    #[test]
    fn list_files_nonexistent_workspace() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();

        // Don't create workspace — list should return empty
        let files = mgr.list_files(wf, run, None).expect("list");
        assert!(files.is_empty());
    }

    // ── Write / Read / Delete ─────────────────────────────────────────

    #[test]
    fn write_file_creates_parents() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();
        mgr.create_run_workspace(wf, run).expect("create");

        mgr.write_file(wf, run, "src/deep/file.txt".as_ref(), b"hello")
            .expect("write");
        let content = mgr
            .read_file(wf, run, "src/deep/file.txt".as_ref())
            .expect("read");
        assert_eq!(content, Some(b"hello".to_vec()));
    }

    #[test]
    fn write_file_overwrites() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();
        mgr.create_run_workspace(wf, run).expect("create");

        mgr.write_file(wf, run, "out.txt".as_ref(), b"first")
            .expect("write1");
        mgr.write_file(wf, run, "out.txt".as_ref(), b"second")
            .expect("write2");
        let content = mgr.read_file(wf, run, "out.txt".as_ref()).expect("read");
        assert_eq!(content, Some(b"second".to_vec()));
    }

    #[test]
    fn delete_file_removes() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();
        mgr.create_run_workspace(wf, run).expect("create");

        mgr.write_file(wf, run, "gone.txt".as_ref(), b"bye")
            .expect("write");
        let removed = mgr
            .delete_file(wf, run, "gone.txt".as_ref())
            .expect("delete");
        assert!(removed);
        let content = mgr.read_file(wf, run, "gone.txt".as_ref()).expect("read");
        assert!(content.is_none());
    }

    #[test]
    fn delete_file_nonexistent_returns_false() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();
        mgr.create_run_workspace(wf, run).expect("create");

        let removed = mgr
            .delete_file(wf, run, "nope.txt".as_ref())
            .expect("delete");
        assert!(!removed);
    }

    #[test]
    fn read_file_nonexistent_returns_none() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();
        mgr.create_run_workspace(wf, run).expect("create");

        let content = mgr
            .read_file(wf, run, "missing.txt".as_ref())
            .expect("read");
        assert!(content.is_none());
    }

    #[test]
    fn read_base_files_selective() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run = Uuid::new_v4();
        mgr.create_run_workspace(wf, run).expect("create");

        // Write 5 files
        for i in 0..5 {
            mgr.write_file(
                wf,
                run,
                format!("file{i}.txt").as_ref(),
                format!("content{i}").as_bytes(),
            )
            .expect("write");
        }

        // Request only 2
        let mut needed = std::collections::HashSet::new();
        needed.insert(std::path::PathBuf::from("file1.txt"));
        needed.insert(std::path::PathBuf::from("file3.txt"));

        let result = mgr.read_base_files(wf, run, &needed).expect("read_base");
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get(&std::path::PathBuf::from("file1.txt")).unwrap(),
            b"content1"
        );
        assert_eq!(
            result.get(&std::path::PathBuf::from("file3.txt")).unwrap(),
            b"content3"
        );
    }

    // ── Isolation ───────────────────────────────────────────────────────

    #[test]
    fn multiple_runs_are_isolated() {
        let (mgr, _tmp) = test_manager();
        let wf = Uuid::new_v4();
        let run1 = Uuid::new_v4();
        let run2 = Uuid::new_v4();

        let root1 = mgr.create_run_workspace(wf, run1).expect("create run1");
        let root2 = mgr.create_run_workspace(wf, run2).expect("create run2");

        fs::write(root1.join("from_run1.txt"), "1").unwrap();
        fs::write(root2.join("from_run2.txt"), "2").unwrap();

        let files1 = mgr.list_files(wf, run1, None).expect("list run1");
        let files2 = mgr.list_files(wf, run2, None).expect("list run2");

        assert_eq!(files1.len(), 1);
        assert_eq!(files2.len(), 1);
        assert!(files1.contains(&std::path::PathBuf::from("from_run1.txt")));
        assert!(files2.contains(&std::path::PathBuf::from("from_run2.txt")));
    }
}
