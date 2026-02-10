#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::execution::{ExecutionContext, GitOps};
    use std::process::Command;
    use tempfile::TempDir;

    fn init_git_repo(dir: &TempDir) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .unwrap();
    }

    #[test]
    fn status_empty_repo() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let status = git.status().unwrap();
        assert!(!status.is_dirty);
    }

    #[test]
    fn status_with_untracked_file() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        std::fs::write(tmp.path().join("new_file.txt"), "content").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let status = git.status().unwrap();
        assert!(status.is_dirty);
        assert_eq!(status.untracked.len(), 1);
    }

    #[test]
    fn create_and_checkout_branch() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Need at least one commit
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let branch_info = git.create_and_checkout_branch("feature/test").unwrap();
        assert_eq!(
            git.current_branch().unwrap(),
            Some("feature/test".to_string())
        );
        // Verify parent branch is tracked
        assert!(branch_info.parent_branch.is_some());
        assert_eq!(branch_info.name, "feature/test");
        assert!(!branch_info.base_commit.is_empty());
    }

    #[test]
    fn invalid_branch_name_rejected() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        assert!(git.validate_branch_name("valid-name").is_ok());
        assert!(git.validate_branch_name("feature/foo").is_ok());
        assert!(git.validate_branch_name("-invalid").is_err());
        assert!(git.validate_branch_name("has space").is_err());
        assert!(git.validate_branch_name("has..dots").is_err());
    }

    #[test]
    fn commit_with_staged_changes() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create and stage a file
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        git.add_files(&["file.txt"]).unwrap();
        let commit = git.commit("test: add file").unwrap();

        assert!(!commit.hash.is_empty());
        assert!(!commit.short_hash.is_empty());
    }

    #[test]
    fn commit_without_staged_changes_fails() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let result = git.commit("empty commit");
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn diff_shows_unstaged_changes() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create, commit, then modify
        std::fs::write(tmp.path().join("file.txt"), "original").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        std::fs::write(tmp.path().join("file.txt"), "modified").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let diff = git.diff().unwrap();
        assert!(diff.contains("-original"));
        assert!(diff.contains("+modified"));
    }

    #[test]
    fn diff_staged_shows_cached_changes() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create and commit
        std::fs::write(tmp.path().join("file.txt"), "original").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        // Modify and stage
        std::fs::write(tmp.path().join("file.txt"), "modified").unwrap();
        Command::new("git")
            .args(["add", "file.txt"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let diff = git.diff_staged().unwrap();
        assert!(diff.contains("-original"));
        assert!(diff.contains("+modified"));

        // Unstaged diff should be empty
        let unstaged = git.diff().unwrap();
        assert!(unstaged.trim().is_empty());
    }

    #[test]
    fn force_push_rejected() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let result = git.push_with_options(PushOptions {
            force: true,
            ..Default::default()
        });

        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    // ========================================================================
    // Merge Operation Tests (Ticket 7.6)
    // ========================================================================

    fn get_default_branch(dir: &TempDir) -> String {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn create_conflicting_branches(dir: &TempDir) {
        // Create a file and commit on main
        std::fs::write(dir.path().join("file.txt"), "main content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "main commit"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Get the default branch name (may be master or main)
        let default_branch = get_default_branch(dir);

        // Create branch with different content
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::fs::write(dir.path().join("file.txt"), "feature content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feature commit"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Go back to main/master and make conflicting change
        Command::new("git")
            .args(["checkout", &default_branch])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::fs::write(dir.path().join("file.txt"), "different main content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "main conflict"])
            .current_dir(dir.path())
            .output()
            .unwrap();
    }

    #[test]
    fn merge_detects_conflicts() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());

        if let MergeResult::Conflict { conflicting_files } = result {
            assert!(conflicting_files.contains(&PathBuf::from("file.txt")));
        }
    }

    #[test]
    fn merge_fast_forward() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create initial commit
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        // Get default branch name
        let default_branch = get_default_branch(&tmp);

        // Create branch with new commit
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::fs::write(tmp.path().join("new.txt"), "new").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feature"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        // Go back to main
        Command::new("git")
            .args(["checkout", &default_branch])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let result = git.merge("feature").unwrap();
        assert!(result.is_success());

        if let MergeResult::Success { fast_forward, .. } = result {
            assert!(fast_forward);
        }
    }

    #[test]
    fn abort_merge_cancels() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        // Create conflict
        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());
        assert!(git.is_merging().unwrap());

        // Abort
        git.abort_merge().unwrap();
        assert!(!git.is_merging().unwrap());
    }

    #[test]
    fn reset_hard_requires_confirmation() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create commit
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        // Without confirmation, should fail
        let result = git.reset_hard("HEAD~1", false);
        assert!(result.is_err());

        // With confirmation, should work
        let result = git.reset_hard("HEAD", true);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_simple_conflict() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let conflict_content = r#"some code
<<<<<<< HEAD
our changes
=======
their changes
>>>>>>> feature
more code"#;

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let regions = git.parse_conflict_markers(conflict_content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].ours, "our changes");
        assert_eq!(regions[0].theirs, "their changes");
        assert!(regions[0].base.is_none());
    }

    #[test]
    fn parse_3way_conflict() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let conflict_content = r#"<<<<<<< HEAD
our version
||||||| merged common ancestors
original version
=======
their version
>>>>>>> feature"#;

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let regions = git.parse_conflict_markers(conflict_content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].ours, "our version");
        assert_eq!(regions[0].theirs, "their version");
        assert_eq!(regions[0].base.as_ref().unwrap(), "original version");
    }

    #[test]
    fn resolve_conflict_ours() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        // Create conflict
        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());

        // Resolve with ours
        git.resolve_conflict(Path::new("file.txt"), ConflictResolution::Ours)
            .unwrap();

        // Should be resolved
        assert!(git.all_conflicts_resolved().unwrap());

        // Complete merge
        let commit = git.complete_merge().unwrap();
        assert!(!commit.hash.is_empty());
    }

    fn setup_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();
        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        (temp_dir, repo_path)
    }

    fn make_initial_commit(repo_path: &Path) {
        std::fs::write(repo_path.join("init.txt"), "init").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(repo_path)
            .output()
            .unwrap();
    }

    #[test]
    fn current_branch_returns_default() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let branch = git.current_branch().unwrap();
        assert!(branch.is_some());
        // Default branch is either "main" or "master"
        let name = branch.unwrap();
        assert!(name == "main" || name == "master");
    }

    #[test]
    fn list_branches_includes_created() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.create_branch("branch-a").unwrap();
        git.create_branch("branch-b").unwrap();

        let branches = git.list_branches().unwrap();
        assert!(branches.contains(&"branch-a".to_string()));
        assert!(branches.contains(&"branch-b".to_string()));
        assert!(branches.len() >= 3); // default + a + b
    }

    #[test]
    fn delete_branch_removes_it() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.create_branch("to-delete").unwrap();

        let branches_before = git.list_branches().unwrap();
        assert!(branches_before.contains(&"to-delete".to_string()));

        git.delete_branch("to-delete").unwrap();

        let branches_after = git.list_branches().unwrap();
        assert!(!branches_after.contains(&"to-delete".to_string()));
    }

    #[test]
    fn delete_current_branch_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let current = git.current_branch().unwrap().unwrap();
        let result = git.delete_branch(&current);
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn add_files_stages_specific_files() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        std::fs::write(repo_path.join("a.txt"), "a").unwrap();
        std::fs::write(repo_path.join("b.txt"), "b").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.add_files(&["a.txt"]).unwrap();

        let status = git.status().unwrap();
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].path, PathBuf::from("a.txt"));
        // b.txt should still be untracked
        assert!(status.untracked.contains(&PathBuf::from("b.txt")));
    }

    #[test]
    fn add_files_empty_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.add_files(&[]);
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn add_all_stages_everything() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        std::fs::write(repo_path.join("a.txt"), "a").unwrap();
        std::fs::write(repo_path.join("b.txt"), "b").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.add_all().unwrap();

        let status = git.status().unwrap();
        assert_eq!(status.staged.len(), 2);
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn diff_stat_shows_changes() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        // Modify tracked file
        std::fs::write(repo_path.join("init.txt"), "modified").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let stat = git.diff_stat().unwrap();
        assert!(stat.contains("init.txt"));
    }

    #[test]
    fn diff_files_between_commits() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));

        // Get first commit hash
        let first = git
            .run_git(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();

        // Make second commit
        std::fs::write(repo_path.join("new.txt"), "new").unwrap();
        git.add_all().unwrap();
        git.commit("add new file").unwrap();

        let second = git
            .run_git(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();

        let files = git.diff_files(&first, &second).unwrap();
        assert!(files.contains(&"new.txt".to_string()));
    }

    #[test]
    fn amend_commit_changes_message() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let amended = git.amend_commit(Some("amended message")).unwrap();
        assert_eq!(amended.message, "amended message");
    }

    #[test]
    fn amend_commit_no_edit() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let amended = git.amend_commit(None).unwrap();
        assert_eq!(amended.message, "initial");
    }

    #[test]
    fn reset_soft_keeps_changes_staged() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));

        // Make second commit
        std::fs::write(repo_path.join("second.txt"), "second").unwrap();
        git.add_all().unwrap();
        git.commit("second commit").unwrap();

        // Soft reset back one commit
        git.reset_soft("HEAD~1").unwrap();

        let status = git.status().unwrap();
        // File should be staged after soft reset
        assert!(!status.staged.is_empty());
        assert!(status
            .staged
            .iter()
            .any(|f| f.path == PathBuf::from("second.txt")));
    }

    #[test]
    fn reset_mixed_unstages_changes() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));

        // Make second commit
        std::fs::write(repo_path.join("second.txt"), "second").unwrap();
        git.add_all().unwrap();
        git.commit("second commit").unwrap();

        // Mixed reset back one commit
        git.reset("HEAD~1").unwrap();

        let status = git.status().unwrap();
        // File should be untracked (not staged) after mixed reset
        assert!(status.staged.is_empty());
        assert!(status.untracked.contains(&PathBuf::from("second.txt")));
    }

    #[test]
    fn has_unpushed_commits_no_upstream() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        // No upstream configured, should return false
        let result = git.has_unpushed_commits().unwrap();
        assert!(!result);
    }

    #[test]
    fn parse_branch_line_with_tracking() {
        let status = parse_porcelain_status("## main...origin/main\n");
        assert_eq!(status.branch, Some("main".to_string()));
    }

    #[test]
    fn parse_branch_line_no_tracking() {
        let status = parse_porcelain_status("## feature-branch\n");
        assert_eq!(status.branch, Some("feature-branch".to_string()));
    }

    #[test]
    fn parse_branch_line_detached_head() {
        let status = parse_porcelain_status("## HEAD (no branch)\n");
        assert_eq!(status.branch, None);
    }

    #[test]
    fn parse_change_type_all_variants_existing() {
        assert_eq!(parse_change_type('A'), ChangeType::Added);
        assert_eq!(parse_change_type('M'), ChangeType::Modified);
        assert_eq!(parse_change_type('D'), ChangeType::Deleted);
        assert_eq!(parse_change_type('R'), ChangeType::Renamed);
        assert_eq!(parse_change_type('C'), ChangeType::Copied);
        assert_eq!(parse_change_type('X'), ChangeType::Unknown);
    }

    #[test]
    fn not_a_repo_error() {
        let tmp = TempDir::new().unwrap();
        // Don't init git
        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        let result = git.status();
        assert!(matches!(result, Err(GitError::NotARepo { .. })));
    }

    #[test]
    fn validate_commit_message_empty_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.validate_commit_message("   ");
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn validate_branch_name_edge_cases() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        assert!(git.validate_branch_name("").is_err());
        assert!(git.validate_branch_name("has~tilde").is_err());
        assert!(git.validate_branch_name("has^caret").is_err());
        assert!(git.validate_branch_name("has:colon").is_err());
        assert!(git.validate_branch_name("ends.").is_err());
        assert!(git.validate_branch_name("ends/").is_err());
        assert!(git.validate_branch_name("has@{ref").is_err());
        assert!(git.validate_branch_name("has[bracket").is_err());
        assert!(git.validate_branch_name("ok-name").is_ok());
    }

    #[test]
    fn status_with_staged_and_unstaged_changes() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        // Modify tracked file and stage it
        std::fs::write(repo_path.join("init.txt"), "modified").unwrap();
        Command::new("git")
            .args(["add", "init.txt"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Modify it again so there are both staged and unstaged changes
        std::fs::write(repo_path.join("init.txt"), "modified again").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let status = git.status().unwrap();

        assert!(status.is_dirty);
        assert!(!status.staged.is_empty());
        assert!(!status.unstaged.is_empty());
        assert_eq!(status.staged[0].change_type, ChangeType::Modified);
        assert_eq!(status.unstaged[0].change_type, ChangeType::Modified);
    }

    #[test]
    fn status_with_deleted_file() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        // Delete the tracked file and stage the deletion
        std::fs::remove_file(repo_path.join("init.txt")).unwrap();
        Command::new("git")
            .args(["add", "init.txt"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let status = git.status().unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].change_type, ChangeType::Deleted);
    }

    #[test]
    fn status_branch_name_populated() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let status = git.status().unwrap();
        assert!(status.branch.is_some());
    }

    #[test]
    fn checkout_branch_switches() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.create_branch("other").unwrap();
        git.checkout_branch("other").unwrap();

        assert_eq!(git.current_branch().unwrap(), Some("other".to_string()));
    }

    #[test]
    fn checkout_branch_invalid_name() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.checkout_branch("-bad");
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn create_branch_invalid_name() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.create_branch("has space");
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn create_and_checkout_branch_invalid_name() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.create_and_checkout_branch("has space");
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn diff_with_options_base_commit() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));
        let first = git
            .run_git(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();

        std::fs::write(repo_path.join("new.txt"), "new").unwrap();
        git.add_all().unwrap();
        git.commit("second").unwrap();

        let diff = git
            .diff_with_options(DiffOptions {
                base_commit: Some(first),
                ..Default::default()
            })
            .unwrap();
        assert!(diff.contains("new.txt"));
    }

    #[test]
    fn diff_with_options_paths_filter() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        std::fs::write(repo_path.join("init.txt"), "changed").unwrap();
        std::fs::write(repo_path.join("other.txt"), "other").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.add_all().unwrap();

        let diff = git
            .diff_with_options(DiffOptions {
                staged: true,
                paths: vec!["init.txt".to_string()],
                ..Default::default()
            })
            .unwrap();
        assert!(diff.contains("init.txt"));
    }

    #[test]
    fn diff_commit_shows_changes() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));

        std::fs::write(repo_path.join("another.txt"), "data").unwrap();
        git.add_all().unwrap();
        git.commit("add another").unwrap();

        let hash = git
            .run_git(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        let diff = git.diff_commit(&hash).unwrap();
        assert!(diff.contains("another.txt"));
    }

    #[test]
    fn commit_message_validation_accepts_valid() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        assert!(git.validate_commit_message("feat: add feature").is_ok());
    }

    #[test]
    fn commit_message_long_first_line_still_valid() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let long_msg = "a".repeat(120);
        // Should still succeed (just a warning, not an error)
        assert!(git.validate_commit_message(&long_msg).is_ok());
    }

    #[test]
    fn not_a_repo_various_operations() {
        let tmp = TempDir::new().unwrap();
        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));

        assert!(matches!(
            git.current_branch(),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.list_branches(),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.create_branch("x"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.create_and_checkout_branch("x"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.checkout_branch("x"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.delete_branch("x"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.add_files(&["x"]),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.add_all(), Err(GitError::NotARepo { .. })));
        assert!(matches!(git.commit("msg"), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.amend_commit(None),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.diff(), Err(GitError::NotARepo { .. })));
        assert!(matches!(git.diff_staged(), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.diff_commit("HEAD"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.diff_files("a", "b"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.diff_stat(), Err(GitError::NotARepo { .. })));
        assert!(matches!(git.push(), Err(GitError::NotARepo { .. })));
        assert!(matches!(git.pull(), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.fetch("origin"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.fetch_remote("origin"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.fetch_refspec("origin", "main"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.fetch_pr("origin", 1),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.merge("main"), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.get_conflicting_files(),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.is_merging(), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.get_conflict_info(Path::new("x")),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.resolve_conflict(Path::new("x"), ConflictResolution::Ours),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.resolve_all_conflicts(ConflictResolution::Ours),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.resolve_conflict_manual(Path::new("x"), "c"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.mark_resolved(Path::new("x")),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.all_conflicts_resolved(),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.complete_merge(),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.abort_merge(), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.reset_hard("HEAD", true),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.clean_working_tree(true),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.reset_soft("HEAD"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.reset("HEAD"), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.has_unpushed_commits(),
            Err(GitError::NotARepo { .. })
        ));
    }

    #[test]
    fn merge_result_methods() {
        let success = MergeResult::Success {
            merge_commit: None,
            fast_forward: true,
        };
        assert!(success.is_success());
        assert!(!success.has_conflicts());

        let conflict = MergeResult::Conflict {
            conflicting_files: vec![],
        };
        assert!(!conflict.is_success());
        assert!(conflict.has_conflicts());

        let failed = MergeResult::Failed {
            reason: "bad".to_string(),
        };
        assert!(!failed.is_success());
        assert!(!failed.has_conflicts());
    }

    #[test]
    fn is_merging_false_normally() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(!git.is_merging().unwrap());
    }

    #[test]
    fn complete_merge_fails_when_not_merging() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.complete_merge();
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn abort_merge_fails_when_not_merging() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.abort_merge();
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn resolve_conflict_theirs() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());

        git.resolve_conflict(Path::new("file.txt"), ConflictResolution::Theirs)
            .unwrap();
        assert!(git.all_conflicts_resolved().unwrap());

        let commit = git.complete_merge().unwrap();
        assert!(!commit.hash.is_empty());

        // Verify theirs content won
        let content = std::fs::read_to_string(tmp.path().join("file.txt")).unwrap();
        assert_eq!(content, "feature content");
    }

    #[test]
    fn resolve_all_conflicts_returns_count() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create initial commit with two files
        std::fs::write(tmp.path().join("a.txt"), "main a").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "main b").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let default_branch = get_default_branch(&tmp);

        // Feature branch modifies both
        Command::new("git")
            .args(["checkout", "-b", "feat"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::fs::write(tmp.path().join("a.txt"), "feat a").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "feat b").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feat"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        // Main branch modifies both differently
        Command::new("git")
            .args(["checkout", &default_branch])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::fs::write(tmp.path().join("a.txt"), "main a v2").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "main b v2").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "main v2"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        let result = git.merge("feat").unwrap();
        assert!(result.has_conflicts());

        let count = git.resolve_all_conflicts(ConflictResolution::Ours).unwrap();
        assert_eq!(count, 2);
        assert!(git.all_conflicts_resolved().unwrap());
    }

    #[test]
    fn resolve_conflict_manual_with_custom_content() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());

        git.resolve_conflict_manual(Path::new("file.txt"), "custom resolution")
            .unwrap();
        assert!(git.all_conflicts_resolved().unwrap());

        let content = std::fs::read_to_string(tmp.path().join("file.txt")).unwrap();
        assert_eq!(content, "custom resolution");
    }

    #[test]
    fn get_conflict_info_parses_markers() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());

        let info = git.get_conflict_info(Path::new("file.txt")).unwrap();
        assert_eq!(info.path, PathBuf::from("file.txt"));
        assert!(!info.regions.is_empty());
        // The conflict should have ours and theirs content
        assert!(!info.regions[0].ours.is_empty());
        assert!(!info.regions[0].theirs.is_empty());
    }

    #[test]
    fn parse_multiple_conflict_regions() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let content = "before\n<<<<<<< HEAD\nours1\n=======\ntheirs1\n>>>>>>> b\nmiddle\n<<<<<<< HEAD\nours2\n=======\ntheirs2\n>>>>>>> b\nafter";
        let regions = git.parse_conflict_markers(content).unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].ours, "ours1");
        assert_eq!(regions[0].theirs, "theirs1");
        assert_eq!(regions[1].ours, "ours2");
        assert_eq!(regions[1].theirs, "theirs2");
    }

    #[test]
    fn parse_conflict_no_markers() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let regions = git
            .parse_conflict_markers("just normal content\nno conflicts")
            .unwrap();
        assert!(regions.is_empty());
    }

    #[test]
    fn clean_working_tree_requires_confirmation() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.clean_working_tree(false);
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn clean_working_tree_removes_changes() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        // Add untracked file and modify tracked file
        std::fs::write(repo_path.join("untracked.txt"), "junk").unwrap();
        std::fs::write(repo_path.join("init.txt"), "modified").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));
        git.clean_working_tree(true).unwrap();

        let status = git.status().unwrap();
        assert!(!status.is_dirty);
        assert!(!repo_path.join("untracked.txt").exists());
    }

    #[test]
    fn fetch_on_local_repo() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        // fetch with no remote configured should fail with CommandFailed
        let result = git.fetch("origin");
        assert!(matches!(result, Err(GitError::CommandFailed { .. })));
    }

    #[test]
    fn fetch_remote_no_remote() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.fetch_remote("origin");
        assert!(matches!(result, Err(GitError::CommandFailed { .. })));
    }

    #[test]
    fn fetch_refspec_no_remote() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.fetch_refspec("origin", "main");
        assert!(matches!(result, Err(GitError::CommandFailed { .. })));
    }

    #[test]
    fn fetch_pr_no_remote() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.fetch_pr("origin", 42);
        assert!(matches!(result, Err(GitError::CommandFailed { .. })));
    }

    #[test]
    fn push_with_options_force_rejected() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.push_with_options(PushOptions {
            force: true,
            ..Default::default()
        });
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn push_no_remote_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.push();
        // Should fail because no remote is configured
        assert!(result.is_err());
    }

    #[test]
    fn pull_no_remote_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.pull();
        assert!(result.is_err());
    }

    #[test]
    fn diff_options_default() {
        let opts = DiffOptions::default();
        assert!(opts.paths.is_empty());
        assert!(!opts.staged);
        assert!(opts.base_commit.is_none());
        assert!(opts.context_lines.is_none());
    }

    #[test]
    fn push_options_default() {
        let opts = PushOptions::default();
        assert_eq!(opts.remote, "origin");
        assert!(opts.branch.is_none());
        assert!(!opts.set_upstream);
        assert!(!opts.force);
    }

    #[test]
    fn get_conflicting_files_empty_when_no_conflicts() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let files = git.get_conflicting_files().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn complete_merge_fails_with_unresolved_conflicts() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        git.merge("feature").unwrap();
        assert!(git.is_merging().unwrap());

        // Try to complete without resolving
        let result = git.complete_merge();
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn validate_branch_name_backslash() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(git.validate_branch_name("has\\backslash").is_err());
    }

    #[test]
    fn validate_branch_name_tab() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(git.validate_branch_name("has\ttab").is_err());
    }

    #[test]
    fn validate_branch_name_newline() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(git.validate_branch_name("has\nnewline").is_err());
    }

    #[test]
    fn validate_branch_name_question_mark() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(git.validate_branch_name("has?question").is_err());
    }

    #[test]
    fn validate_branch_name_asterisk() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(git.validate_branch_name("has*star").is_err());
    }

    #[test]
    fn mark_resolved_stages_file() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        git.merge("feature").unwrap();

        // Write resolved content manually
        std::fs::write(tmp.path().join("file.txt"), "resolved").unwrap();
        git.mark_resolved(Path::new("file.txt")).unwrap();

        assert!(git.all_conflicts_resolved().unwrap());
    }

    #[test]
    fn push_with_set_upstream_option() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        // Will fail due to no remote, but exercises the set_upstream code path
        let result = git.push_with_options(PushOptions {
            set_upstream: true,
            ..Default::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn push_with_custom_branch() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.push_with_options(PushOptions {
            branch: Some("custom-branch".to_string()),
            ..Default::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn conflict_region_line_numbers() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let content = "line1\nline2\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> b\nline8";
        let regions = git.parse_conflict_markers(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start_line, 3);
        assert_eq!(regions[0].end_line, 7);
    }

    #[test]
    fn pull_from_no_remote_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.pull_from("origin", "main");
        assert!(result.is_err());
    }

    // ── parse_porcelain_status (standalone parser) ───────────────────────────

    #[test]
    fn parse_porcelain_status_empty() {
        let status = parse_porcelain_status("");
        assert!(status.staged.is_empty());
        assert!(status.unstaged.is_empty());
        assert!(status.untracked.is_empty());
        assert!(!status.is_dirty);
        assert!(status.branch.is_none());
    }

    #[test]
    fn parse_porcelain_status_branch_only() {
        let output = "## main...origin/main\n";
        let status = parse_porcelain_status(output);
        assert_eq!(status.branch, Some("main".to_string()));
        assert!(!status.is_dirty);
    }

    #[test]
    fn parse_porcelain_status_detached_head() {
        let output = "## HEAD (no branch)\n";
        let status = parse_porcelain_status(output);
        assert_eq!(status.branch, None);
    }

    #[test]
    fn parse_porcelain_status_staged_modified() {
        let output = "## main\nM  src/lib.rs\n";
        let status = parse_porcelain_status(output);
        assert_eq!(status.staged.len(), 1);
        assert_eq!(
            status.staged[0].path,
            std::path::PathBuf::from("src/lib.rs")
        );
        assert_eq!(status.staged[0].change_type, ChangeType::Modified);
        assert!(status.is_dirty);
    }

    #[test]
    fn parse_porcelain_status_unstaged_modified() {
        let output = "## main\n M src/lib.rs\n";
        let status = parse_porcelain_status(output);
        assert!(status.staged.is_empty());
        assert_eq!(status.unstaged.len(), 1);
        assert_eq!(status.unstaged[0].change_type, ChangeType::Modified);
    }

    #[test]
    fn parse_porcelain_status_untracked() {
        let output = "## main\n?? new_file.txt\n";
        let status = parse_porcelain_status(output);
        assert!(status.staged.is_empty());
        assert!(status.unstaged.is_empty());
        assert_eq!(status.untracked.len(), 1);
        assert_eq!(
            status.untracked[0],
            std::path::PathBuf::from("new_file.txt")
        );
        assert!(status.is_dirty);
    }

    #[test]
    fn parse_porcelain_status_added_file() {
        let output = "## main\nA  new_file.rs\n";
        let status = parse_porcelain_status(output);
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].change_type, ChangeType::Added);
    }

    #[test]
    fn parse_porcelain_status_deleted_file() {
        let output = "## main\nD  old_file.rs\n";
        let status = parse_porcelain_status(output);
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].change_type, ChangeType::Deleted);
    }

    #[test]
    fn parse_porcelain_status_mixed() {
        let output = "## feature-branch...origin/feature-branch\nM  src/lib.rs\n M src/main.rs\nA  new.rs\n?? untracked.txt\n";
        let status = parse_porcelain_status(output);
        assert_eq!(status.branch, Some("feature-branch".to_string()));
        assert_eq!(status.staged.len(), 2); // M + A
        assert_eq!(status.unstaged.len(), 1); // M in worktree
        assert_eq!(status.untracked.len(), 1);
        assert!(status.is_dirty);
    }
}
