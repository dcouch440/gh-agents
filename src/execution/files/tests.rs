#[cfg(test)]
mod tests {
    use super::super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn read_file_success() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        let content = ops.read_file("test.txt").await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn read_file_outside_project_rejected() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        let result = ops.read_file("../../../etc/passwd").await;
        assert!(matches!(result, Err(FileError::PathOutsideProject { .. })));
    }

    #[tokio::test]
    async fn read_file_not_found() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        let result = ops.read_file("nonexistent.txt").await;
        assert!(matches!(result, Err(FileError::NotFound { .. })));
    }

    #[tokio::test]
    async fn write_file_creates_new() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        ops.write_file("new_file.txt", "content").await.unwrap();

        let content = std::fs::read_to_string(tmp.path().join("new_file.txt")).unwrap();
        assert_eq!(content, "content");
    }

    #[tokio::test]
    async fn write_file_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        ops.write_file("deep/nested/dir/file.txt", "content")
            .await
            .unwrap();

        assert!(tmp.path().join("deep/nested/dir/file.txt").exists());
    }

    #[tokio::test]
    async fn write_file_outside_project_rejected() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        let result = ops.write_file("../../escape.txt", "bad").await;
        assert!(matches!(result, Err(FileError::PathOutsideProject { .. })));
    }

    #[tokio::test]
    async fn exists_returns_true_for_existing_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("exists.txt"), "content").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        assert!(ops.exists("exists.txt").await.unwrap());
    }

    #[tokio::test]
    async fn exists_returns_false_for_missing_file() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        assert!(!ops.exists("missing.txt").await.unwrap());
    }

    #[tokio::test]
    async fn delete_file_removes_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("to_delete.txt"), "content").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        ops.delete_file("to_delete.txt").await.unwrap();

        assert!(!tmp.path().join("to_delete.txt").exists());
    }

    #[tokio::test]
    async fn list_tree_walks_subdirectories_and_marks_them() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("README.md"), "").unwrap();
        std::fs::write(tmp.path().join("src/main.rs"), "").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        let (files, dropped) = ops.list_tree(".", 3, 500).await.unwrap();
        assert_eq!(files, vec!["README.md", "src/", "src/main.rs"]);
        assert_eq!(dropped, 0);
    }

    #[tokio::test]
    async fn list_tree_errors_on_a_missing_directory() {
        // An empty listing for a path that is not there reads as "the step
        // produced nothing", which is the wrong conclusion.
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        let result = ops.list_tree("nope", 3, 500).await;
        assert!(matches!(result, Err(FileError::NotFound { .. })));
    }

    #[tokio::test]
    async fn list_tree_errors_when_the_path_is_a_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        let err = ops.list_tree("a.txt", 3, 500).await.unwrap_err();
        assert!(err.to_string().contains("not a directory"), "{err}");
    }

    #[tokio::test]
    async fn list_tree_skips_a_subdirectory_it_cannot_read() {
        // One unreadable directory should cost the caller that directory, not
        // the whole listing.
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("visible.txt"), "").unwrap();
        let locked = tmp.path().join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::write(locked.join("hidden.txt"), "").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);
        let result = ops.list_tree(".", 3, 500).await;

        // Restore before asserting so TempDir can always clean itself up.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        let (files, _) = result.unwrap();
        assert!(files.contains(&"visible.txt".to_string()), "{files:?}");
        assert!(files.contains(&"locked/".to_string()), "{files:?}");
        assert!(
            !files.contains(&"locked/hidden.txt".to_string()),
            "{files:?}"
        );
    }

    #[tokio::test]
    async fn list_dir_returns_entries() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("file1.txt"), "").unwrap();
        std::fs::write(tmp.path().join("file2.txt"), "").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let ops = FileOps::new(ctx);

        let entries = ops.list_dir(".").await.unwrap();
        assert_eq!(entries.len(), 2);
    }
}
