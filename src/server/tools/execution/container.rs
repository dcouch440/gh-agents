//! Container-aware tool implementations.
//!
//! Each function executes a tool inside a persistent Docker container
//! via `ContainerHandle`.

use serde_json::{json, Value};

use crate::execution::{
    parse_porcelain_status, validate_container_path, ContainerHandle, LIST_FILES_MAX_DEPTH,
};

use super::file_io::{edit_file_core, ContainerFileIO};
use super::{list_files_response, DEFAULT_LIST_DEPTH};
use crate::server::tools::shared::{is_tool_allowed, tool_not_allowed_error};

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
    if !is_tool_allowed(name, allowed_tools) {
        return tool_not_allowed_error(name);
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

/// Default line budget for `read_file`. It bypasses the stdout truncation that
/// wraps `run_command`, so an unbounded read of a large file lands whole in the
/// context budget — and `ContextBudgetExceeded` is a hard error, not a
/// skippable one.
const DEFAULT_READ_LIMIT: usize = 2000;

/// Line-bounded window over a file's contents, as `read_file` returns it.
///
/// Split out from the tool body so the boundary cases are testable without a
/// container: an empty file, an offset past the end, a zero limit, and the
/// byte-exactness the read/modify/write round trip depends on.
pub(super) fn read_file_window(path: &str, content: &str, offset: usize, limit: usize) -> Value {
    // A zero limit would return nothing while still advertising `next_offset`
    // equal to the offset just asked for — a read loop that never terminates.
    let limit = if limit == 0 {
        DEFAULT_READ_LIMIT
    } else {
        limit
    };
    // `split_inclusive` keeps each line's terminator, so the window
    // concatenates back to the exact bytes read. `lines()` dropped the `\r` of
    // a CRLF file and the file's trailing newline, which turned the
    // read/modify/write_file round trip the tool descriptions encourage into a
    // silent rewrite of the whole file.
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let total = lines.len();

    // Offset 0 into an empty file is a legitimate empty read; any other offset
    // past the end is not. Guarding on `total` alone let `offset > 0` reach the
    // slice below with `end == 0` and panic, which the workforce executor turns
    // into `Agent task panicked` and fails the whole step.
    if offset > 0 && offset >= total {
        return json!({
            "error": format!(
                "offset {} is past the end of {} ({} lines)",
                offset, path, total
            )
        });
    }

    let end = offset.saturating_add(limit).min(total);
    let mut out = json!({
        "content": lines[offset..end].concat(),
        "total_lines": total,
        "line_range": [offset + 1, end],
    });

    if end < total {
        out["next_offset"] = json!(end);
        out["note"] = json!(format!(
            "{} of {} lines shown. Continue with offset={}.",
            end - offset,
            total,
            end
        ));
    }
    out
}

async fn container_read_file(input: &Value, handle: &ContainerHandle) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => p,
        None => return json!({ "error": "Missing required parameter: path" }),
    };
    if let Err(e) = validate_container_path(path) {
        return json!({ "error": e.to_string() });
    }
    let offset = input["offset"].as_u64().unwrap_or(0) as usize;
    let limit = input["limit"].as_u64().unwrap_or(DEFAULT_READ_LIMIT as u64) as usize;

    match handle.read_file(path).await {
        Ok(content) => read_file_window(path, &content, offset, limit),
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
    // One cheap `test -f` before the write. Two things depend on it: the
    // Created/Modified classification the diagnostics bridge needs (write_file
    // never touches the shell, so the snapshot diff never sees it), and the
    // overwrite signal the agent needs — `cat >` said nothing about clobbering
    // an upstream deliverable, which is how run dd27d008 destroyed one.
    let existed = handle
        .exec_shell(&format!(
            "test -f {}",
            crate::execution::container::shell_escape_path(path)
        ))
        .await
        .map(|r| r.success)
        .unwrap_or(false);

    match handle.write_file(path, content).await {
        Ok(()) => {
            let mut out = json!({
                "success": true,
                "path": path,
                "bytes": content.len(),
                "lines": content.lines().count(),
                "overwrote": existed,
            });
            if existed {
                out["warning"] = json!(format!(
                    "{} already existed and was replaced. If it came from an upstream \
                     agent, save under a new name instead.",
                    path
                ));
            }
            out
        }
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
    // Clamped here rather than only inside `list_files` so the depth the
    // response reports is the depth the listing was actually taken at.
    let depth = input["depth"]
        .as_u64()
        .map(|d| d as u32)
        .unwrap_or(DEFAULT_LIST_DEPTH)
        .clamp(1, LIST_FILES_MAX_DEPTH);

    match handle.list_files(path, depth).await {
        Ok((files, dropped)) => list_files_response(path, files, dropped, depth),
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
    // Some models emit HTML-encoded shell operators in tool inputs.
    // Unescape via shared helper so all entity handling stays in one place.
    let command = crate::execution::diagnostics::html_unescape(command);

    // The system-node designer writes its JSON through this path, which has no
    // diagnostics engine behind it. Same guard, same reason: a fragment whose
    // heredoc never closed writes a truncated file and reports success.
    let open = crate::execution::diagnostics::pre::heredoc::unterminated_heredocs(&command);
    if !open.is_empty() {
        return json!({
            "error": format!(
                "Command was cut off before its heredoc closed ({}). Not executed — running \
                 it would have written a truncated file that reported success.",
                open.join(", ")
            ),
            "hint": "The heredoc body is bounded by your output limit. Write the file in \
                     smaller pieces."
        });
    }

    match handle.exec_shell(&command).await {
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
