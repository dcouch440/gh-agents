//! Headless mode for non-interactive operation
//!
//! Enables CI/CD integration and scripting by providing a non-TUI interface.

use crate::cli::Args;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

/// Task input from file or command line
#[derive(Debug, Clone, Deserialize)]
pub struct TaskInput {
    /// Task description
    pub description: String,
    /// Priority level (optional)
    #[serde(default)]
    pub priority: Option<String>,
    /// Associated GitHub issue URL (optional)
    #[serde(default)]
    pub github_issue: Option<String>,
}

/// Result of processing a task
#[derive(Debug, Clone)]
pub enum TaskResult {
    /// Task completed successfully
    Success { message: String },
    /// Task failed
    Failed { error: String },
    /// Task was skipped
    Skipped { reason: String },
}

/// Headless mode runner
pub struct HeadlessRunner {
    args: Args,
    output: Box<dyn Write + Send>,
}

impl HeadlessRunner {
    /// Create a new headless runner
    pub fn new(args: Args) -> Result<Self> {
        let output: Box<dyn Write + Send> = match &args.output {
            Some(path) => {
                let file = File::create(path)
                    .context(format!("failed to create output file: {:?}", path))?;
                Box::new(BufWriter::new(file))
            }
            None => Box::new(io::stdout()),
        };

        Ok(Self { args, output })
    }

    /// Write a line to output
    fn write_line(&mut self, line: &str) -> Result<()> {
        writeln!(self.output, "{}", line)?;
        self.output.flush()?;
        Ok(())
    }

    /// Write a progress message
    fn write_progress(&mut self, message: &str) -> Result<()> {
        self.write_line(&format!("[PROGRESS] {}", message))
    }

    /// Write an error message
    fn write_error(&mut self, message: &str) -> Result<()> {
        self.write_line(&format!("[ERROR] {}", message))
    }

    /// Write a result message
    fn write_result(&mut self, status: &str, details: &str) -> Result<()> {
        self.write_line(&format!("[RESULT] {} - {}", status, details))
    }

    /// Parse input file into tasks
    fn parse_input_file(path: &PathBuf) -> Result<Vec<TaskInput>> {
        let content = std::fs::read_to_string(path).context("failed to read input file")?;

        // Try JSON array first
        if let Ok(tasks) = serde_json::from_str::<Vec<TaskInput>>(&content) {
            return Ok(tasks);
        }

        // Try single JSON object
        if let Ok(task) = serde_json::from_str::<TaskInput>(&content) {
            return Ok(vec![task]);
        }

        // Fall back to line-by-line plain text
        let tasks = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| !line.starts_with('#')) // Skip comments
            .map(|line| TaskInput {
                description: line.trim().to_string(),
                priority: None,
                github_issue: None,
            })
            .collect();

        Ok(tasks)
    }

    /// Run the headless session
    pub async fn run(mut self) -> Result<()> {
        self.write_line("nexor headless mode starting...")?;

        let tasks: Vec<TaskInput> = if let Some(ref input_path) = self.args.input {
            Self::parse_input_file(input_path)?
        } else if let Some(ref task) = self.args.task {
            vec![TaskInput {
                description: task.clone(),
                priority: None,
                github_issue: None,
            }]
        } else if let Some(ref url) = self.args.sync {
            let url = url.clone();
            self.write_progress(&format!("Syncing from GitHub: {}", url))?;
            vec![TaskInput {
                description: format!("Work on GitHub issue: {}", url),
                priority: None,
                github_issue: Some(url),
            }]
        } else {
            anyhow::bail!("no task specified");
        };

        self.write_line(&format!("Processing {} task(s)...", tasks.len()))?;

        let mut failed = 0;
        let mut succeeded = 0;

        for (i, task) in tasks.iter().enumerate() {
            self.write_line(&format!("\n--- Task {}/{} ---", i + 1, tasks.len()))?;
            self.write_progress(&format!("Task: {}", task.description))?;

            if let Some(ref issue) = task.github_issue {
                self.write_progress(&format!("GitHub issue: {}", issue))?;
            }

            match self.process_task(task).await {
                Ok(TaskResult::Success { message }) => {
                    self.write_result("SUCCESS", &message)?;
                    succeeded += 1;
                }
                Ok(TaskResult::Skipped { reason }) => {
                    self.write_result("SKIPPED", &reason)?;
                }
                Ok(TaskResult::Failed { error }) => {
                    self.write_error(&error)?;
                    self.write_result("FAILED", &task.description)?;
                    failed += 1;
                }
                Err(e) => {
                    self.write_error(&format!("{}", e))?;
                    self.write_result("FAILED", &task.description)?;
                    failed += 1;
                }
            }
        }

        self.write_line(&format!(
            "\n=== Summary: {}/{} tasks completed successfully ===",
            succeeded,
            tasks.len()
        ))?;

        if failed > 0 {
            self.write_line(&format!("{} task(s) failed", failed))?;
            std::process::exit(1);
        }

        Ok(())
    }

    /// Process a single task
    async fn process_task(&mut self, task: &TaskInput) -> Result<TaskResult> {
        self.write_progress("Initializing...")?;

        // TODO: Connect to actual orchestration system
        // For now, this is a placeholder implementation
        self.write_progress("Decomposing task...")?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        self.write_progress("Assigning to agents...")?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        self.write_progress("Executing...")?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Placeholder: mark as success
        // In real implementation, this would interact with orchestrator
        Ok(TaskResult::Success {
            message: format!("Completed: {}", task.description),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_text_tasks() {
        let content = "Task 1\nTask 2\n# Comment\nTask 3\n";
        let path = std::env::temp_dir().join("test_tasks.txt");
        std::fs::write(&path, content).unwrap();

        let tasks = HeadlessRunner::parse_input_file(&path).unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].description, "Task 1");
        assert_eq!(tasks[1].description, "Task 2");
        assert_eq!(tasks[2].description, "Task 3");

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn parse_json_array_tasks() {
        let content = r#"[
            {"description": "Task 1", "priority": "high"},
            {"description": "Task 2"}
        ]"#;
        let path = std::env::temp_dir().join("test_tasks.json");
        std::fs::write(&path, content).unwrap();

        let tasks = HeadlessRunner::parse_input_file(&path).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].description, "Task 1");
        assert_eq!(tasks[0].priority, Some("high".to_string()));
        assert_eq!(tasks[1].description, "Task 2");
        assert!(tasks[1].priority.is_none());

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn parse_json_single_task() {
        let content = r#"{"description": "Single task"}"#;
        let path = std::env::temp_dir().join("test_single_task.json");
        std::fs::write(&path, content).unwrap();

        let tasks = HeadlessRunner::parse_input_file(&path).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].description, "Single task");

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn task_input_deserializes() {
        let json =
            r#"{"description": "Test", "priority": "high", "github_issue": "https://example.com"}"#;
        let task: TaskInput = serde_json::from_str(json).unwrap();
        assert_eq!(task.description, "Test");
        assert_eq!(task.priority, Some("high".to_string()));
        assert_eq!(task.github_issue, Some("https://example.com".to_string()));
    }

    #[test]
    fn task_input_minimal() {
        let json = r#"{"description": "Test"}"#;
        let task: TaskInput = serde_json::from_str(json).unwrap();
        assert_eq!(task.description, "Test");
        assert!(task.priority.is_none());
        assert!(task.github_issue.is_none());
    }

    #[test]
    fn headless_runner_write_methods() {
        let args = Args {
            headless: true,
            port: 3000,
            task: Some("test".into()),
            input: None,
            output: None,
            config: None,
            verbose: 0,
            sync: None,
        };
        let buf: Vec<u8> = Vec::new();
        let mut runner = HeadlessRunner {
            args,
            output: Box::new(buf),
        };

        runner.write_line("hello").unwrap();
        runner.write_progress("working").unwrap();
        runner.write_error("oops").unwrap();
        runner.write_result("OK", "done").unwrap();
    }

    #[test]
    fn task_result_variants() {
        let success = TaskResult::Success {
            message: "ok".into(),
        };
        let failed = TaskResult::Failed {
            error: "err".into(),
        };
        let skipped = TaskResult::Skipped {
            reason: "skip".into(),
        };

        match success {
            TaskResult::Success { message } => assert_eq!(message, "ok"),
            _ => panic!("expected Success"),
        }
        match failed {
            TaskResult::Failed { error } => assert_eq!(error, "err"),
            _ => panic!("expected Failed"),
        }
        match skipped {
            TaskResult::Skipped { reason } => assert_eq!(reason, "skip"),
            _ => panic!("expected Skipped"),
        }
    }

    #[test]
    fn parse_input_file_empty() {
        let path = std::env::temp_dir().join("test_empty_input.txt");
        std::fs::write(&path, "").unwrap();
        let tasks = HeadlessRunner::parse_input_file(&path).unwrap();
        assert!(tasks.is_empty());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn parse_input_file_only_comments() {
        let path = std::env::temp_dir().join("test_comments_input.txt");
        std::fs::write(&path, "# comment 1\n# comment 2\n\n").unwrap();
        let tasks = HeadlessRunner::parse_input_file(&path).unwrap();
        assert!(tasks.is_empty());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn headless_runner_new_with_output_file() {
        let tmp = std::env::temp_dir().join("nexor_test_output.txt");
        let args = Args {
            headless: true,
            port: 3000,
            task: Some("test".into()),
            input: None,
            output: Some(tmp.clone()),
            config: None,
            verbose: 0,
            sync: None,
        };
        let runner = HeadlessRunner::new(args).unwrap();
        // File should have been created
        assert!(tmp.exists());
        drop(runner);
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn write_methods_produce_expected_prefixes() {
        let buf: Vec<u8> = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let args = Args {
            headless: true,
            port: 3000,
            task: Some("test".into()),
            input: None,
            output: None,
            config: None,
            verbose: 0,
            sync: None,
        };
        let mut runner = HeadlessRunner {
            args,
            output: Box::new(cursor),
        };

        runner.write_line("plain line").unwrap();
        runner.write_progress("step 1").unwrap();
        runner.write_error("something broke").unwrap();
        runner.write_result("SUCCESS", "all good").unwrap();

        // Downcast to read buffer contents
        let _output = runner.output.as_mut() as *mut dyn Write;
        // We can't easily downcast, so instead re-create with a shared buffer
        drop(runner);

        // Use a shared buffer approach
        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let buf_clone = buf.clone();

        struct SharedBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Write for SharedBuf {
            fn write(&mut self, data: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().write(data)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let args2 = Args {
            headless: true,
            port: 3000,
            task: Some("test".into()),
            input: None,
            output: None,
            config: None,
            verbose: 0,
            sync: None,
        };
        let mut runner2 = HeadlessRunner {
            args: args2,
            output: Box::new(SharedBuf(buf_clone)),
        };

        runner2.write_line("plain line").unwrap();
        runner2.write_progress("step 1").unwrap();
        runner2.write_error("something broke").unwrap();
        runner2.write_result("SUCCESS", "all good").unwrap();

        let contents = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(contents.contains("plain line"));
        assert!(contents.contains("[PROGRESS] step 1"));
        assert!(contents.contains("[ERROR] something broke"));
        assert!(contents.contains("[RESULT] SUCCESS - all good"));
    }

    #[test]
    fn headless_runner_new_with_invalid_output_path() {
        let args = Args {
            headless: true,
            port: 3000,
            task: Some("test".into()),
            input: None,
            output: Some("/nonexistent/dir/that/does/not/exist/output.txt".into()),
            config: None,
            verbose: 0,
            sync: None,
        };
        let result = HeadlessRunner::new(args);
        match result {
            Err(e) => assert!(e.to_string().contains("failed to create output file")),
            Ok(_) => panic!("expected error for invalid output path"),
        }
    }

    #[test]
    fn parse_input_file_nonexistent() {
        let path = PathBuf::from("/nonexistent/file.txt");
        let result = HeadlessRunner::parse_input_file(&path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("failed to read input file"));
    }

    #[test]
    fn parse_input_file_with_whitespace_lines() {
        let path = std::env::temp_dir().join("test_whitespace_input.txt");
        std::fs::write(&path, "  Task 1  \n\n  \n  Task 2  \n").unwrap();
        let tasks = HeadlessRunner::parse_input_file(&path).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].description, "Task 1");
        assert_eq!(tasks[1].description, "Task 2");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn parse_input_file_json_with_github_issue() {
        let content =
            r#"{"description": "Fix bug", "github_issue": "https://github.com/org/repo/issues/1"}"#;
        let path = std::env::temp_dir().join("test_github_issue.json");
        std::fs::write(&path, content).unwrap();
        let tasks = HeadlessRunner::parse_input_file(&path).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(
            tasks[0].github_issue,
            Some("https://github.com/org/repo/issues/1".to_string())
        );
        std::fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn run_with_task_arg() {
        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let buf_clone = buf.clone();

        struct SharedBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Write for SharedBuf {
            fn write(&mut self, data: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().write(data)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let args = Args {
            headless: true,
            port: 3000,
            task: Some("do something".into()),
            input: None,
            output: None,
            config: None,
            verbose: 0,
            sync: None,
        };
        let runner = HeadlessRunner {
            args,
            output: Box::new(SharedBuf(buf_clone)),
        };

        runner.run().await.unwrap();

        let contents = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(contents.contains("nexor headless mode starting..."));
        assert!(contents.contains("Processing 1 task(s)..."));
        assert!(contents.contains("[PROGRESS] Task: do something"));
        assert!(contents.contains("[RESULT] SUCCESS"));
        assert!(contents.contains("1/1 tasks completed successfully"));
    }

    #[tokio::test]
    async fn run_with_sync_arg() {
        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let buf_clone = buf.clone();

        struct SharedBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Write for SharedBuf {
            fn write(&mut self, data: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().write(data)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let args = Args {
            headless: true,
            port: 3000,
            task: None,
            input: None,
            output: None,
            config: None,
            verbose: 0,
            sync: Some("https://github.com/org/repo/issues/42".into()),
        };
        let runner = HeadlessRunner {
            args,
            output: Box::new(SharedBuf(buf_clone)),
        };

        runner.run().await.unwrap();

        let contents = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(contents.contains("Syncing from GitHub"));
        assert!(contents.contains("https://github.com/org/repo/issues/42"));
    }

    #[tokio::test]
    async fn run_with_no_task_fails() {
        let args = Args {
            headless: true,
            port: 3000,
            task: None,
            input: None,
            output: None,
            config: None,
            verbose: 0,
            sync: None,
        };
        let runner = HeadlessRunner {
            args,
            output: Box::new(Vec::<u8>::new()),
        };

        let result = runner.run().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no task specified"));
    }

    #[tokio::test]
    async fn run_with_input_file() {
        let path = std::env::temp_dir().join("test_run_input.json");
        std::fs::write(
            &path,
            r#"[{"description": "task A"}, {"description": "task B"}]"#,
        )
        .unwrap();

        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let buf_clone = buf.clone();

        struct SharedBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Write for SharedBuf {
            fn write(&mut self, data: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().write(data)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let args = Args {
            headless: true,
            port: 3000,
            task: None,
            input: Some(path.clone()),
            output: None,
            config: None,
            verbose: 0,
            sync: None,
        };
        let runner = HeadlessRunner {
            args,
            output: Box::new(SharedBuf(buf_clone)),
        };

        runner.run().await.unwrap();

        let contents = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(contents.contains("Processing 2 task(s)..."));
        assert!(contents.contains("Task 1/2"));
        assert!(contents.contains("Task 2/2"));
        assert!(contents.contains("2/2 tasks completed successfully"));
        std::fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn run_with_input_file_containing_github_issues() {
        let path = std::env::temp_dir().join("test_run_github_input.json");
        std::fs::write(
            &path,
            r#"[{"description": "fix issue", "github_issue": "https://github.com/org/repo/issues/1"}]"#,
        )
        .unwrap();

        let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let buf_clone = buf.clone();

        struct SharedBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Write for SharedBuf {
            fn write(&mut self, data: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().write(data)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let args = Args {
            headless: true,
            port: 3000,
            task: None,
            input: Some(path.clone()),
            output: None,
            config: None,
            verbose: 0,
            sync: None,
        };
        let runner = HeadlessRunner {
            args,
            output: Box::new(SharedBuf(buf_clone)),
        };

        runner.run().await.unwrap();

        let contents = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(contents.contains("[PROGRESS] GitHub issue: https://github.com/org/repo/issues/1"));
        std::fs::remove_file(path).ok();
    }

    #[tokio::test]
    async fn run_writes_to_output_file() {
        let output_path = std::env::temp_dir().join("nexor_test_run_output.txt");
        let args = Args {
            headless: true,
            port: 3000,
            task: Some("file output test".into()),
            input: None,
            output: Some(output_path.clone()),
            config: None,
            verbose: 0,
            sync: None,
        };
        let runner = HeadlessRunner::new(args).unwrap();
        runner.run().await.unwrap();

        let contents = std::fs::read_to_string(&output_path).unwrap();
        assert!(contents.contains("nexor headless mode starting..."));
        assert!(contents.contains("[RESULT] SUCCESS"));
        std::fs::remove_file(output_path).ok();
    }

    #[tokio::test]
    async fn process_task_returns_success() {
        let buf: Vec<u8> = Vec::new();
        let args = Args {
            headless: true,
            port: 3000,
            task: Some("test".into()),
            input: None,
            output: None,
            config: None,
            verbose: 0,
            sync: None,
        };
        let mut runner = HeadlessRunner {
            args,
            output: Box::new(buf),
        };

        let task = TaskInput {
            description: "Do something".to_string(),
            priority: None,
            github_issue: None,
        };

        let result = runner.process_task(&task).await.unwrap();
        match result {
            TaskResult::Success { message } => {
                assert!(message.contains("Do something"));
            }
            _ => panic!("expected Success"),
        }
    }
}
