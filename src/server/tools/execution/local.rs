//! Local filesystem tool implementations.
//!
//! Each function executes a tool against the host filesystem using
//! `ExecutionContext` for project root resolution.

use serde_json::{json, Value};

use crate::execution::{ExecutionContext, FileOps, GitOps, Sandbox, TestRunner};

use super::file_io::{edit_file_core, LocalFileIO};

pub(super) async fn exec_read_file(input: &Value, ctx: &ExecutionContext) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => p,
        None => return json!({ "error": "Missing required parameter: path" }),
    };
    let file_ops = FileOps::new(ctx.clone());
    let full_path = ctx.project_root.join(path);
    match file_ops.read_file(&full_path).await {
        Ok(content) => json!({ "content": content }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(super) async fn exec_write_file(input: &Value, ctx: &ExecutionContext) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => p,
        None => return json!({ "error": "Missing required parameter: path" }),
    };
    let content = match input["content"].as_str() {
        Some(c) => c,
        None => return json!({ "error": "Missing required parameter: content" }),
    };
    let file_ops = FileOps::new(ctx.clone());
    let full_path = ctx.project_root.join(path);
    match file_ops.write_file(&full_path, content).await {
        Ok(()) => json!({ "success": true, "path": path, "bytes": content.len() }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(super) async fn exec_edit_file(input: &Value, ctx: &ExecutionContext) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => p,
        None => return json!({ "error": "Missing required parameter: path" }),
    };
    let old_string = match input["old_string"].as_str() {
        Some(s) => s,
        None => return json!({ "error": "Missing required parameter: old_string" }),
    };
    let new_string = match input["new_string"].as_str() {
        Some(s) => s,
        None => return json!({ "error": "Missing required parameter: new_string" }),
    };

    let io = LocalFileIO {
        file_ops: FileOps::new(ctx.clone()),
        ctx,
    };
    edit_file_core(path, old_string, new_string, &io).await
}

pub(super) async fn exec_list_files(input: &Value, ctx: &ExecutionContext) -> Value {
    let path = input["path"].as_str().unwrap_or(".");
    let file_ops = FileOps::new(ctx.clone());
    let full_path = ctx.project_root.join(path);
    match file_ops.list_dir(&full_path).await {
        Ok(entries) => {
            let names: Vec<String> = entries
                .iter()
                .filter_map(|p| {
                    p.strip_prefix(&ctx.project_root)
                        .ok()
                        .map(|rel| rel.to_string_lossy().to_string())
                })
                .collect();
            json!({ "files": names })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(super) fn exec_git_status(ctx: &ExecutionContext) -> Value {
    let git = GitOps::new(ctx.clone());
    match git.status() {
        Ok(status) => json!({
            "branch": status.branch,
            "staged": status.staged.iter().map(|f| json!({
                "path": f.path,
                "change_type": format!("{:?}", f.change_type)
            })).collect::<Vec<_>>(),
            "unstaged": status.unstaged.iter().map(|f| json!({
                "path": f.path,
                "change_type": format!("{:?}", f.change_type)
            })).collect::<Vec<_>>(),
            "untracked": status.untracked,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(super) fn exec_git_diff(input: &Value, ctx: &ExecutionContext) -> Value {
    let git = GitOps::new(ctx.clone());
    let staged = input["staged"].as_bool().unwrap_or(false);
    let result = if staged {
        git.diff_staged()
    } else {
        git.diff()
    };
    match result {
        Ok(diff) => json!({ "diff": diff }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(super) fn exec_git_add(input: &Value, ctx: &ExecutionContext) -> Value {
    let paths = match input["paths"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
        None => return json!({ "error": "Missing required parameter: paths" }),
    };
    let git = GitOps::new(ctx.clone());
    match git.add_files(&paths) {
        Ok(()) => json!({ "success": true, "staged": paths }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(super) fn exec_git_commit(input: &Value, ctx: &ExecutionContext) -> Value {
    let message = match input["message"].as_str() {
        Some(m) => m,
        None => return json!({ "error": "Missing required parameter: message" }),
    };
    let git = GitOps::new(ctx.clone());
    match git.commit(message) {
        Ok(info) => json!({
            "success": true,
            "sha": info.hash,
            "message": info.message,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub(super) fn exec_git_branch(input: &Value, ctx: &ExecutionContext) -> Value {
    let git = GitOps::new(ctx.clone());
    if let Some(name) = input["create"].as_str() {
        match git.create_branch(name) {
            Ok(info) => json!({ "created": info.name }),
            Err(e) => json!({ "error": e.to_string() }),
        }
    } else {
        match git.current_branch() {
            Ok(Some(branch)) => json!({ "branch": branch }),
            Ok(None) => json!({ "branch": null, "detached": true }),
            Err(e) => json!({ "error": e.to_string() }),
        }
    }
}

pub(super) async fn exec_run_tests(input: &Value, ctx: &ExecutionContext) -> Value {
    let mut runner = TestRunner::new(ctx.clone());
    runner.detect_framework();

    if let Some(pattern) = input["pattern"].as_str() {
        match runner.run_specific(pattern).await {
            Ok(result) => test_result_to_json(&result),
            Err(e) => json!({ "error": e.to_string() }),
        }
    } else {
        match runner.run_tests().await {
            Ok(result) => test_result_to_json(&result),
            Err(e) => json!({ "error": e.to_string() }),
        }
    }
}

fn test_result_to_json(result: &crate::execution::TestResult) -> Value {
    json!({
        "passed": result.passed,
        "failed": result.failed,
        "skipped": result.skipped,
        "success": result.success,
        "duration_ms": result.duration_ms,
        "stdout": result.stdout,
        "stderr": result.stderr,
    })
}

pub(super) async fn exec_run_command(input: &Value, ctx: &ExecutionContext) -> Value {
    let command = match input["command"].as_str() {
        Some(c) => crate::execution::diagnostics::html_unescape(c),
        None => return json!({ "error": "Missing required parameter: command" }),
    };
    let sandbox = Sandbox::with_defaults(ctx.clone());
    match sandbox.exec_shell(&command).await {
        Ok(result) => json!({
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "success": result.exit_code == 0,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}
