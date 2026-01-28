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
}
