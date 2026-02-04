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
