//! Container-aware tool implementations.
//!
//! Each function executes a tool inside a persistent Docker container
//! via `ContainerHandle`.

use serde_json::{json, Value};

use crate::execution::{parse_porcelain_status, validate_container_path, ContainerHandle};

use super::file_io::{edit_file_core, ContainerFileIO};

/// Execute a tool inside a persistent Docker container.
///
/// File/git/command tools delegate to `ContainerHandle`; host-only tools
/// (create_doc, search_docs, update_doc) are executed locally.
pub async fn execute_tool_in_container(
    name: &str,
    input: &Value,
    handle: &ContainerHandle,
    allowed_tools: Option<&[String]>,
) -> Value {
    if let Some(allowed) = allowed_tools {
        if !allowed.iter().any(|t| t == name) {
            return json!({ "error": format!("Tool '{}' is not allowed for this agent", name) });
        }
    }

    match name {
        "read_file" => container_read_file(input, handle).await,
        "write_file" => container_write_file(input, handle).await,
        "edit_file" => container_edit_file(input, handle).await,
        "list_files" => container_list_files(input, handle).await,
        "git_status" => container_git_status(handle).await,
        "git_diff" => container_git_diff(input, handle).await,
        "git_add" => container_git_add(input, handle).await,
        "git_commit" => container_git_commit(input, handle).await,
        "git_branch" => container_git_branch(input, handle).await,
        "run_tests" => container_run_tests(input, handle).await,
        "run_command" => container_run_command(input, handle).await,
        "create_doc" | "search_docs" | "update_doc" => {
            json!({ "error": format!("Tool '{}' is not supported in container mode", name) })
        }
        _ => json!({ "error": format!("Unknown tool: {}", name) }),
    }
}

// ── Container tool implementations ────────────────────────────────────────

async fn container_read_file(input: &Value, handle: &ContainerHandle) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => p,
        None => return json!({ "error": "Missing required parameter: path" }),
    };
    if let Err(e) = validate_container_path(path) {
        return json!({ "error": e.to_string() });
    }
    match handle.read_file(path).await {
        Ok(content) => json!({ "content": content }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn container_write_file(input: &Value, handle: &ContainerHandle) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => p,
        None => return json!({ "error": "Missing required parameter: path" }),
    };
    if let Err(e) = validate_container_path(path) {
        return json!({ "error": e.to_string() });
    }
    let content = match input["content"].as_str() {
        Some(c) => c,
        None => return json!({ "error": "Missing required parameter: content" }),
    };
    match handle.write_file(path, content).await {
        Ok(()) => json!({ "success": true, "path": path }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn container_edit_file(input: &Value, handle: &ContainerHandle) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => p,
        None => return json!({ "error": "Missing required parameter: path" }),
    };
    if let Err(e) = validate_container_path(path) {
        return json!({ "error": e.to_string() });
    }
    let old_string = match input["old_string"].as_str() {
        Some(s) => s,
        None => return json!({ "error": "Missing required parameter: old_string" }),
    };
    let new_string = match input["new_string"].as_str() {
        Some(s) => s,
        None => return json!({ "error": "Missing required parameter: new_string" }),
    };

    let io = ContainerFileIO { handle };
    edit_file_core(path, old_string, new_string, &io).await
}

async fn container_list_files(input: &Value, handle: &ContainerHandle) -> Value {
    let path = input["path"].as_str().unwrap_or(".");
    if path != "." {
        if let Err(e) = validate_container_path(path) {
            return json!({ "error": e.to_string() });
        }
    }
    match handle.list_files(path).await {
        Ok(files) => json!({ "files": files }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn container_git_status(handle: &ContainerHandle) -> Value {
    match handle.git(&["status", "--porcelain=v1", "-b"]).await {
        Ok(output) => git_status_to_json(&parse_porcelain_status(&output)),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

/// Convert a parsed `GitStatus` to a JSON value for tool output.
fn git_status_to_json(status: &crate::execution::GitStatus) -> Value {
    let staged: Vec<Value> = status
        .staged
        .iter()
        .map(|f| json!({ "path": f.path.display().to_string(), "change_type": format!("{:?}", f.change_type) }))
        .collect();
    let unstaged: Vec<Value> = status
        .unstaged
        .iter()
        .map(|f| json!({ "path": f.path.display().to_string(), "change_type": format!("{:?}", f.change_type) }))
        .collect();
    let untracked: Vec<String> = status
        .untracked
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    json!({
        "branch": status.branch.as_deref().unwrap_or(""),
        "staged": staged,
        "unstaged": unstaged,
        "untracked": untracked,
    })
}

async fn container_git_diff(input: &Value, handle: &ContainerHandle) -> Value {
    let staged = input["staged"].as_bool().unwrap_or(false);
    let args = if staged {
        vec!["diff", "--cached"]
    } else {
        vec!["diff"]
    };
    match handle.git(&args).await {
        Ok(diff) => json!({ "diff": diff }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn container_git_add(input: &Value, handle: &ContainerHandle) -> Value {
    let paths = match input["paths"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
        None => return json!({ "error": "Missing required parameter: paths" }),
    };
    let mut args = vec!["add", "--"];
    args.extend(paths.iter().copied());
    match handle.git(&args).await {
        Ok(_) => json!({ "success": true, "staged": paths }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn container_git_commit(input: &Value, handle: &ContainerHandle) -> Value {
    let message = match input["message"].as_str() {
        Some(m) => m,
        None => return json!({ "error": "Missing required parameter: message" }),
    };
    match handle.git(&["commit", "-m", message]).await {
        Ok(output) => {
            // Parse commit SHA from git output: "[branch hash] message"
            let sha = output
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .map(|s| s.trim_end_matches(']'))
                .unwrap_or("unknown");
            json!({
                "success": true,
                "sha": sha,
                "message": message,
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn container_git_branch(input: &Value, handle: &ContainerHandle) -> Value {
    if let Some(name) = input["create"].as_str() {
        match handle.git(&["checkout", "-b", name]).await {
            Ok(_) => json!({ "created": name }),
            Err(e) => json!({ "error": e.to_string() }),
        }
    } else {
        match handle.git(&["rev-parse", "--abbrev-ref", "HEAD"]).await {
            Ok(branch) => json!({ "branch": branch.trim() }),
            Err(e) => json!({ "error": e.to_string() }),
        }
    }
}

async fn container_run_command(input: &Value, handle: &ContainerHandle) -> Value {
    let command = match input["command"].as_str() {
        Some(c) => c,
        None => return json!({ "error": "Missing required parameter: command" }),
    };
    match handle.exec_shell(command).await {
        Ok(result) => json!({
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "success": result.success,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn container_run_tests(input: &Value, handle: &ContainerHandle) -> Value {
    // In container mode, run_tests just shells out to the test command
    let pattern = input["pattern"].as_str();
    let command = match pattern {
        Some(p) => format!("cargo test {} 2>&1 || npm test -- {} 2>&1", p, p),
        None => "cargo test 2>&1 || npm test 2>&1".to_string(),
    };
    match handle.exec_shell(&command).await {
        Ok(result) => json!({
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "success": result.success,
            "duration_ms": result.duration_ms,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}
