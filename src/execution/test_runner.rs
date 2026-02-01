//! Test runner with framework auto-detection

use crate::execution::ExecutionContext;
use std::path::Path;
use std::process::Stdio;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("no test framework detected")]
    NoFrameworkDetected,

    #[error("test command failed: {0}")]
    CommandFailed(String),

    #[error("failed to execute: {0}")]
    ExecutionError(#[from] std::io::Error),

    #[error("failed to parse test output: {0}")]
    ParseError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestFramework {
    /// Rust - cargo test
    Cargo,
    /// Node.js - npm test
    Npm,
    /// Node.js - jest
    Jest,
    /// Python - pytest
    Pytest,
    /// Python - unittest
    PythonUnittest,
    /// Go - go test
    Go,
    /// Generic - custom command
    Generic,
}

impl TestFramework {
    /// Get the default command for this framework
    pub fn default_command(&self) -> Vec<&'static str> {
        match self {
            TestFramework::Cargo => vec!["cargo", "test"],
            TestFramework::Npm => vec!["npm", "test"],
            TestFramework::Jest => vec!["npx", "jest"],
            TestFramework::Pytest => vec!["pytest", "-v"],
            TestFramework::PythonUnittest => vec!["python", "-m", "unittest", "discover"],
            TestFramework::Go => vec!["go", "test", "./..."],
            TestFramework::Generic => vec!["make", "test"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub framework: TestFramework,
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub passed: Option<u32>,
    pub failed: Option<u32>,
    pub skipped: Option<u32>,
    pub duration_ms: u64,
}

impl TestResult {
    /// Check if any tests failed
    pub fn has_failures(&self) -> bool {
        self.failed.map(|f| f > 0).unwrap_or(!self.success)
    }

    /// Get a summary string
    pub fn summary(&self) -> String {
        let status = if self.success { "PASSED" } else { "FAILED" };

        match (self.passed, self.failed, self.skipped) {
            (Some(p), Some(f), Some(s)) => {
                format!("{}: {} passed, {} failed, {} skipped ({} ms)", status, p, f, s, self.duration_ms)
            }
            (Some(p), Some(f), None) => {
                format!("{}: {} passed, {} failed ({} ms)", status, p, f, self.duration_ms)
            }
            _ => {
                format!("{} ({} ms)", status, self.duration_ms)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub test_name: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub stack_trace: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TestOutputEvent {
    Line(String),
    Progress { passed: u32, failed: u32 },
    Complete(TestResult),
    Error(String),
}

pub struct TestRunner {
    ctx: ExecutionContext,
    framework: Option<TestFramework>,
}

impl TestRunner {
    pub fn new(ctx: ExecutionContext) -> Self {
        Self { ctx, framework: None }
    }

    /// Auto-detect the test framework based on project files
    pub fn detect_framework(&mut self) -> Option<TestFramework> {
        if self.framework.is_some() {
            return self.framework;
        }

        let root = &self.ctx.project_root;

        // Check in priority order
        let detected = if root.join("Cargo.toml").exists() {
            Some(TestFramework::Cargo)
        } else if root.join("go.mod").exists() {
            Some(TestFramework::Go)
        } else if root.join("package.json").exists() {
            // Check for jest in package.json
            if self.has_jest_config(root) {
                Some(TestFramework::Jest)
            } else {
                Some(TestFramework::Npm)
            }
        } else if root.join("pytest.ini").exists() || root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
            // Check for pytest
            if self.has_pytest(root) {
                Some(TestFramework::Pytest)
            } else {
                Some(TestFramework::PythonUnittest)
            }
        } else if root.join("Makefile").exists() {
            // Check if Makefile has test target
            if self.makefile_has_test(root) {
                Some(TestFramework::Generic)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(fw) = detected {
            tracing::info!(framework = ?fw, "Detected test framework");
        } else {
            tracing::warn!("No test framework detected");
        }

        self.framework = detected;
        detected
    }

    fn has_jest_config(&self, root: &Path) -> bool {
        // Check for jest.config.js or jest in package.json
        if root.join("jest.config.js").exists() || root.join("jest.config.ts").exists() {
            return true;
        }

        // Check package.json for jest
        if let Ok(content) = std::fs::read_to_string(root.join("package.json")) {
            return content.contains("\"jest\"");
        }

        false
    }

    fn has_pytest(&self, root: &Path) -> bool {
        // Check for pytest in requirements or pyproject.toml
        if root.join("pytest.ini").exists() {
            return true;
        }

        if let Ok(content) = std::fs::read_to_string(root.join("pyproject.toml")) {
            if content.contains("[tool.pytest]") || content.contains("pytest") {
                return true;
            }
        }

        if let Ok(content) = std::fs::read_to_string(root.join("requirements.txt")) {
            return content.lines().any(|l| l.starts_with("pytest"));
        }

        false
    }

    fn makefile_has_test(&self, root: &Path) -> bool {
        if let Ok(content) = std::fs::read_to_string(root.join("Makefile")) {
            // Check for test: target
            content.lines().any(|l| l.starts_with("test:"))
        } else {
            false
        }
    }

    /// Set framework explicitly (override detection)
    pub fn with_framework(mut self, framework: TestFramework) -> Self {
        self.framework = Some(framework);
        self
    }

    /// Run tests using detected or configured framework
    pub async fn run_tests(&mut self) -> Result<TestResult, TestError> {
        let framework = self.detect_framework().ok_or(TestError::NoFrameworkDetected)?;

        self.run_with_command(&framework.default_command()).await
    }

    /// Run tests with a custom command
    pub async fn run_with_command(&self, command: &[&str]) -> Result<TestResult, TestError> {
        let framework = self.framework.unwrap_or(TestFramework::Generic);

        if command.is_empty() {
            return Err(TestError::CommandFailed("Empty command".to_string()));
        }

        let start = std::time::Instant::now();

        tracing::info!(
            command = ?command,
            framework = ?framework,
            "Running tests"
        );

        let output = Command::new(command[0]).args(&command[1..]).current_dir(&self.ctx.project_root).output().await?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        // Parse results based on framework
        let (passed, failed, skipped) = self.parse_results(framework, &stdout, &stderr);

        let result = TestResult {
            framework,
            success,
            exit_code,
            stdout,
            stderr,
            passed,
            failed,
            skipped,
            duration_ms,
        };

        if success {
            tracing::info!(
                passed = ?result.passed,
                failed = ?result.failed,
                duration_ms = result.duration_ms,
                "Tests passed"
            );
        } else {
            tracing::warn!(
                passed = ?result.passed,
                failed = ?result.failed,
                exit_code = exit_code,
                "Tests failed"
            );
        }

        Ok(result)
    }

    /// Run a specific test (by name/pattern)
    pub async fn run_specific(&self, pattern: &str) -> Result<TestResult, TestError> {
        let framework = self.framework.ok_or(TestError::NoFrameworkDetected)?;

        let command: Vec<&str> = match framework {
            TestFramework::Cargo => vec!["cargo", "test", pattern],
            TestFramework::Jest => vec!["npx", "jest", "-t", pattern],
            TestFramework::Pytest => vec!["pytest", "-v", "-k", pattern],
            TestFramework::Npm => vec!["npm", "test", "--", pattern],
            TestFramework::Go => vec!["go", "test", "-run", pattern, "./..."],
            _ => return Err(TestError::CommandFailed("Specific test not supported for this framework".to_string())),
        };

        self.run_with_command(&command).await
    }

    /// Run tests with streaming output
    pub async fn run_tests_streaming(&mut self, output_tx: mpsc::Sender<TestOutputEvent>) -> Result<TestResult, TestError> {
        let framework = self.detect_framework().ok_or(TestError::NoFrameworkDetected)?;

        let command = framework.default_command();

        if command.is_empty() {
            return Err(TestError::CommandFailed("Empty command".to_string()));
        }

        let start = std::time::Instant::now();

        let _ = output_tx.send(TestOutputEvent::Line(format!("Running: {}", command.join(" ")))).await;

        // Spawn process with piped stdout/stderr
        let mut child = Command::new(command[0])
            .args(&command[1..])
            .current_dir(&self.ctx.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| TestError::CommandFailed("Failed to capture stdout".to_string()))?;
        let stderr = child.stderr.take().ok_or_else(|| TestError::CommandFailed("Failed to capture stderr".to_string()))?;

        // Spawn tasks to read stdout and stderr
        let output_tx_clone = output_tx.clone();
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut collected = String::new();

            while let Ok(Some(line)) = lines.next_line().await {
                collected.push_str(&line);
                collected.push('\n');
                let _ = output_tx_clone.send(TestOutputEvent::Line(line)).await;
            }

            collected
        });

        let output_tx_clone = output_tx.clone();
        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut collected = String::new();

            while let Ok(Some(line)) = lines.next_line().await {
                collected.push_str(&line);
                collected.push('\n');
                let _ = output_tx_clone.send(TestOutputEvent::Line(format!("[stderr] {}", line))).await;
            }

            collected
        });

        // Wait for process to complete
        let status = child.wait().await?;

        // Collect output
        let stdout_output = stdout_task.await.unwrap_or_default();
        let stderr_output = stderr_task.await.unwrap_or_default();

        let duration_ms = start.elapsed().as_millis() as u64;
        let exit_code = status.code().unwrap_or(-1);
        let success = status.success();

        let (passed, failed, skipped) = self.parse_results(framework, &stdout_output, &stderr_output);

        let result = TestResult {
            framework,
            success,
            exit_code,
            stdout: stdout_output,
            stderr: stderr_output,
            passed,
            failed,
            skipped,
            duration_ms,
        };

        // Send completion event
        let _ = output_tx.send(TestOutputEvent::Complete(result.clone())).await;

        Ok(result)
    }

    /// Run tests with a timeout
    pub async fn run_tests_with_timeout(&mut self, timeout_secs: u64) -> Result<TestResult, TestError> {
        let result = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), self.run_tests()).await;

        match result {
            Ok(r) => r,
            Err(_) => Err(TestError::CommandFailed(format!("Tests timed out after {} seconds", timeout_secs))),
        }
    }

    fn parse_results(&self, framework: TestFramework, stdout: &str, stderr: &str) -> (Option<u32>, Option<u32>, Option<u32>) {
        match framework {
            TestFramework::Cargo => self.parse_cargo_output(stdout, stderr),
            TestFramework::Jest => self.parse_jest_output(stdout),
            TestFramework::Pytest => self.parse_pytest_output(stdout),
            TestFramework::Go => self.parse_go_output(stdout),
            _ => (None, None, None),
        }
    }

    fn parse_cargo_output(&self, stdout: &str, _stderr: &str) -> (Option<u32>, Option<u32>, Option<u32>) {
        // Look for "test result: ok. X passed; Y failed; Z ignored"
        for line in stdout.lines() {
            if line.starts_with("test result:") {
                let mut passed = None;
                let mut failed = None;
                let mut ignored = None;

                for part in line.split(';') {
                    let part = part.trim();
                    if part.contains("passed") {
                        passed = part.split_whitespace().find_map(|w| w.parse::<u32>().ok());
                    } else if part.contains("failed") {
                        failed = part.split_whitespace().find_map(|w| w.parse::<u32>().ok());
                    } else if part.contains("ignored") {
                        ignored = part.split_whitespace().find_map(|w| w.parse::<u32>().ok());
                    }
                }

                return (passed, failed, ignored);
            }
        }
        (None, None, None)
    }

    fn parse_jest_output(&self, stdout: &str) -> (Option<u32>, Option<u32>, Option<u32>) {
        // Look for "Tests: X passed, Y failed, Z total"
        for line in stdout.lines() {
            if line.contains("Tests:") {
                let mut passed = None;
                let mut failed = None;
                let mut skipped = None;

                for part in line.split(',') {
                    let part = part.trim();
                    if part.contains("passed") {
                        passed = part.split_whitespace().find_map(|w| w.parse::<u32>().ok());
                    } else if part.contains("failed") {
                        failed = part.split_whitespace().find_map(|w| w.parse::<u32>().ok());
                    } else if part.contains("skipped") {
                        skipped = part.split_whitespace().find_map(|w| w.parse::<u32>().ok());
                    }
                }

                return (passed, failed, skipped);
            }
        }
        (None, None, None)
    }

    fn parse_pytest_output(&self, stdout: &str) -> (Option<u32>, Option<u32>, Option<u32>) {
        // Look for "X passed, Y failed, Z skipped"
        for line in stdout.lines().rev() {
            if line.contains("passed") || line.contains("failed") {
                let mut passed = None;
                let mut failed = None;
                let mut skipped = None;

                for part in line.split(',') {
                    let part = part.trim();
                    if part.contains("passed") {
                        passed = part.split_whitespace().find_map(|w| w.parse::<u32>().ok());
                    } else if part.contains("failed") {
                        failed = part.split_whitespace().find_map(|w| w.parse::<u32>().ok());
                    } else if part.contains("skipped") {
                        skipped = part.split_whitespace().find_map(|w| w.parse::<u32>().ok());
                    }
                }

                if passed.is_some() || failed.is_some() {
                    return (passed, failed, skipped);
                }
            }
        }
        (None, None, None)
    }

    fn parse_go_output(&self, stdout: &str) -> (Option<u32>, Option<u32>, Option<u32>) {
        // Count "--- PASS:" and "--- FAIL:" lines
        let passed = stdout.matches("--- PASS:").count() as u32;
        let failed = stdout.matches("--- FAIL:").count() as u32;
        let skipped = stdout.matches("--- SKIP:").count() as u32;

        if passed > 0 || failed > 0 {
            (Some(passed), Some(failed), Some(skipped))
        } else {
            (None, None, None)
        }
    }

    /// Parse failures from cargo test output
    pub fn parse_cargo_failures(&self, stdout: &str) -> Vec<TestFailure> {
        let mut failures = Vec::new();
        let mut current_test: Option<String> = None;
        let mut current_message = String::new();
        let mut in_failure = false;

        for line in stdout.lines() {
            // Detect test start
            if line.starts_with("---- ") && line.ends_with(" stdout ----") {
                let test_name = line.trim_start_matches("---- ").trim_end_matches(" stdout ----").to_string();
                current_test = Some(test_name);
                current_message.clear();
                in_failure = true;
            }
            // Detect failure message
            else if in_failure && line.contains("panicked at") {
                current_message = line.to_string();
            }
            // End of failure block
            else if in_failure && line.starts_with("----") {
                if let Some(test_name) = current_test.take() {
                    failures.push(TestFailure {
                        test_name,
                        message: current_message.clone(),
                        file: None,
                        line: None,
                        stack_trace: None,
                    });
                }
                in_failure = false;
            }
        }

        failures
    }

    /// Extract failures from test result
    pub fn extract_failures(&self, result: &TestResult) -> Vec<TestFailure> {
        match result.framework {
            TestFramework::Cargo => self.parse_cargo_failures(&result.stdout),
            // Add other frameworks as needed
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detects_cargo() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);

        assert_eq!(runner.detect_framework(), Some(TestFramework::Cargo));
    }

    #[test]
    fn detects_npm() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);

        assert_eq!(runner.detect_framework(), Some(TestFramework::Npm));
    }

    #[test]
    fn detects_jest() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("package.json"), r#"{"devDependencies":{"jest":"^29"}}"#).unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);

        assert_eq!(runner.detect_framework(), Some(TestFramework::Jest));
    }

    #[test]
    fn detects_pytest() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("pytest.ini"), "[pytest]").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);

        assert_eq!(runner.detect_framework(), Some(TestFramework::Pytest));
    }

    #[test]
    fn detects_go() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("go.mod"), "module test").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);

        assert_eq!(runner.detect_framework(), Some(TestFramework::Go));
    }

    #[test]
    fn no_framework_detected() {
        let tmp = TempDir::new().unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);

        assert_eq!(runner.detect_framework(), None);
    }

    #[test]
    fn parse_cargo_output() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = "running 5 tests\ntest result: ok. 4 passed; 1 failed; 0 ignored; 0 measured";

        let (passed, failed, ignored) = runner.parse_cargo_output(stdout, "");
        assert_eq!(passed, Some(4));
        assert_eq!(failed, Some(1));
        assert_eq!(ignored, Some(0));
    }

    #[test]
    fn parse_pytest_output() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = "===== 10 passed, 2 failed, 1 skipped in 3.45s =====";

        let (passed, failed, skipped) = runner.parse_pytest_output(stdout);
        assert_eq!(passed, Some(10));
        assert_eq!(failed, Some(2));
        assert_eq!(skipped, Some(1));
    }

    #[test]
    fn parse_cargo_failure() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = r#"
running 2 tests
test tests::passing ... ok
test tests::failing ... FAILED

failures:

---- tests::failing stdout ----
thread 'tests::failing' panicked at 'assertion failed: false', src/lib.rs:10:9
----

failures:
    tests::failing

test result: FAILED. 1 passed; 1 failed; 0 ignored
"#;

        let failures = runner.parse_cargo_failures(stdout);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].test_name, "tests::failing");
        assert!(failures[0].message.contains("panicked"));
    }

    #[test]
    fn test_result_summary() {
        let result = TestResult {
            framework: TestFramework::Cargo,
            success: true,
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            passed: Some(10),
            failed: Some(0),
            skipped: Some(2),
            duration_ms: 1500,
        };

        assert_eq!(result.summary(), "PASSED: 10 passed, 0 failed, 2 skipped (1500 ms)");
    }

    #[test]
    fn test_result_has_failures() {
        let result = TestResult {
            framework: TestFramework::Cargo,
            success: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            passed: Some(5),
            failed: Some(2),
            skipped: None,
            duration_ms: 1000,
        };

        assert!(result.has_failures());
    }

    #[test]
    fn default_commands_all_frameworks() {
        assert_eq!(TestFramework::Cargo.default_command(), vec!["cargo", "test"]);
        assert_eq!(TestFramework::Npm.default_command(), vec!["npm", "test"]);
        assert_eq!(TestFramework::Jest.default_command(), vec!["npx", "jest"]);
        assert_eq!(TestFramework::Pytest.default_command(), vec!["pytest", "-v"]);
        assert_eq!(TestFramework::PythonUnittest.default_command(), vec!["python", "-m", "unittest", "discover"]);
        assert_eq!(TestFramework::Go.default_command(), vec!["go", "test", "./..."]);
        assert_eq!(TestFramework::Generic.default_command(), vec!["make", "test"]);
    }

    #[test]
    fn parse_jest_output_with_results() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = "Tests: 5 passed, 2 failed, 1 skipped, 8 total";
        let (passed, failed, skipped) = runner.parse_jest_output(stdout);
        assert_eq!(passed, Some(5));
        assert_eq!(failed, Some(2));
        assert_eq!(skipped, Some(1));
    }

    #[test]
    fn parse_jest_output_no_matches() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let (p, f, s) = runner.parse_jest_output("no test output here");
        assert!(p.is_none());
        assert!(f.is_none());
        assert!(s.is_none());
    }

    #[test]
    fn parse_go_output_with_results() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = "--- PASS: TestA (0.00s)\n--- PASS: TestB (0.01s)\n--- FAIL: TestC (0.00s)\n--- SKIP: TestD (0.00s)\n";
        let (p, f, s) = runner.parse_go_output(stdout);
        assert_eq!(p, Some(2));
        assert_eq!(f, Some(1));
        assert_eq!(s, Some(1));
    }

    #[test]
    fn parse_go_output_no_results() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let (p, f, s) = runner.parse_go_output("ok  \tpackage\t0.001s");
        assert!(p.is_none());
        assert!(f.is_none());
        assert!(s.is_none());
    }

    #[test]
    fn parse_cargo_output_no_result_line() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let (p, f, i) = runner.parse_cargo_output("running 0 tests", "");
        assert!(p.is_none());
        assert!(f.is_none());
        assert!(i.is_none());
    }

    #[test]
    fn has_failures_with_zero_failed() {
        let result = TestResult {
            framework: TestFramework::Cargo,
            success: true,
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            passed: Some(5),
            failed: Some(0),
            skipped: None,
            duration_ms: 100,
        };
        assert!(!result.has_failures());
    }

    #[test]
    fn has_failures_with_none_failed_not_success() {
        let result = TestResult {
            framework: TestFramework::Generic,
            success: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            passed: None,
            failed: None,
            skipped: None,
            duration_ms: 100,
        };
        assert!(result.has_failures());
    }

    #[test]
    fn summary_passed_failed_only() {
        let result = TestResult {
            framework: TestFramework::Cargo,
            success: true,
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            passed: Some(10),
            failed: Some(0),
            skipped: None,
            duration_ms: 500,
        };
        assert_eq!(result.summary(), "PASSED: 10 passed, 0 failed (500 ms)");
    }

    #[test]
    fn summary_no_counts() {
        let result = TestResult {
            framework: TestFramework::Generic,
            success: false,
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            passed: None,
            failed: None,
            skipped: None,
            duration_ms: 200,
        };
        assert_eq!(result.summary(), "FAILED (200 ms)");
    }

    #[test]
    fn parse_cargo_failures_multiple() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = r#"
---- tests::fail_a stdout ----
thread 'tests::fail_a' panicked at 'assert a', src/lib.rs:1:1
----
---- tests::fail_b stdout ----
thread 'tests::fail_b' panicked at 'assert b', src/lib.rs:2:1
----
"#;
        let failures = runner.parse_cargo_failures(stdout);
        assert_eq!(failures.len(), 2);
        assert_eq!(failures[0].test_name, "tests::fail_a");
        assert_eq!(failures[1].test_name, "tests::fail_b");
    }

    #[test]
    fn parse_cargo_failures_empty() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let failures = runner.parse_cargo_failures("");
        assert!(failures.is_empty());
    }

    #[test]
    fn extract_failures_non_cargo() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let result = TestResult {
            framework: TestFramework::Jest,
            success: false,
            exit_code: 1,
            stdout: "stuff".into(),
            stderr: String::new(),
            passed: None,
            failed: None,
            skipped: None,
            duration_ms: 100,
        };
        assert!(runner.extract_failures(&result).is_empty());
    }

    #[test]
    fn detects_jest_config_ts() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("jest.config.ts"), "").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);
        assert_eq!(runner.detect_framework(), Some(TestFramework::Jest));
    }

    #[test]
    fn detects_pytest_from_pyproject() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("pyproject.toml"), "[tool.pytest]\nminversion = \"6.0\"").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);
        assert_eq!(runner.detect_framework(), Some(TestFramework::Pytest));
    }

    #[test]
    fn detects_makefile_with_test() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Makefile"), "test:\n\t./run_tests.sh\n").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);
        assert_eq!(runner.detect_framework(), Some(TestFramework::Generic));
    }

    #[test]
    fn no_framework_for_makefile_without_test() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Makefile"), "build:\n\tcc main.c\n").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);
        assert_eq!(runner.detect_framework(), None);
    }

    #[test]
    fn detects_setup_py_unittest() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("setup.py"), "from setuptools import setup\nsetup()").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);
        assert_eq!(runner.detect_framework(), Some(TestFramework::PythonUnittest));
    }

    #[test]
    fn with_framework_override() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx).with_framework(TestFramework::Go);
        assert_eq!(runner.framework, Some(TestFramework::Go));
    }

    #[test]
    fn detect_framework_caches_result() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);

        assert_eq!(runner.detect_framework(), Some(TestFramework::Cargo));
        // Remove the file; cached value should still be returned
        std::fs::remove_file(tmp.path().join("Cargo.toml")).unwrap();
        assert_eq!(runner.detect_framework(), Some(TestFramework::Cargo));
    }

    #[test]
    fn parse_results_generic_returns_none() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx).with_framework(TestFramework::Generic);
        let (p, f, s) = runner.parse_results(TestFramework::Generic, "anything", "anything");
        assert!(p.is_none());
        assert!(f.is_none());
        assert!(s.is_none());
    }

    #[test]
    fn parse_results_npm_returns_none() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx).with_framework(TestFramework::Npm);
        let (p, f, s) = runner.parse_results(TestFramework::Npm, "output", "");
        assert!(p.is_none());
        assert!(f.is_none());
        assert!(s.is_none());
    }

    #[test]
    fn parse_results_python_unittest_returns_none() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx).with_framework(TestFramework::PythonUnittest);
        let (p, f, s) = runner.parse_results(TestFramework::PythonUnittest, "output", "");
        assert!(p.is_none());
        assert!(f.is_none());
        assert!(s.is_none());
    }

    #[test]
    fn parse_results_dispatches_to_cargo() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = "test result: ok. 3 passed; 0 failed; 1 ignored";
        let (p, f, s) = runner.parse_results(TestFramework::Cargo, stdout, "");
        assert_eq!(p, Some(3));
        assert_eq!(f, Some(0));
        assert_eq!(s, Some(1));
    }

    #[test]
    fn parse_results_dispatches_to_jest() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = "Tests: 3 passed, 0 failed, 3 total";
        let (p, f, _s) = runner.parse_results(TestFramework::Jest, stdout, "");
        assert_eq!(p, Some(3));
        assert_eq!(f, Some(0));
    }

    #[test]
    fn parse_results_dispatches_to_pytest() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = "===== 5 passed in 1.0s =====";
        let (p, f, s) = runner.parse_results(TestFramework::Pytest, stdout, "");
        assert_eq!(p, Some(5));
        assert_eq!(f, None);
        assert_eq!(s, None);
    }

    #[test]
    fn parse_results_dispatches_to_go() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = "--- PASS: TestX (0.00s)\n";
        let (p, f, s) = runner.parse_results(TestFramework::Go, stdout, "");
        assert_eq!(p, Some(1));
        assert_eq!(f, Some(0));
        assert_eq!(s, Some(0));
    }

    #[test]
    fn has_pytest_from_requirements_txt() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("pyproject.toml"), "[build-system]").unwrap();
        std::fs::write(tmp.path().join("requirements.txt"), "pytest==7.0\nrequests\n").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);
        assert_eq!(runner.detect_framework(), Some(TestFramework::Pytest));
    }

    #[test]
    fn has_pytest_pyproject_with_pytest_keyword() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("pyproject.toml"), "[project]\ndependencies = [\"pytest\"]\n").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);
        assert_eq!(runner.detect_framework(), Some(TestFramework::Pytest));
    }

    #[test]
    fn has_pytest_returns_false_no_pytest_indicators() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("pyproject.toml"), "[build-system]\nrequires = [\"setuptools\"]\n").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);
        assert_eq!(runner.detect_framework(), Some(TestFramework::PythonUnittest));
    }

    #[test]
    fn detects_jest_config_js() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("jest.config.js"), "module.exports = {}").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);
        assert_eq!(runner.detect_framework(), Some(TestFramework::Jest));
    }

    #[test]
    fn extract_failures_cargo() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let result = TestResult {
            framework: TestFramework::Cargo,
            success: false,
            exit_code: 1,
            stdout: "---- my_test stdout ----\nthread panicked at 'oops'\n----\n".into(),
            stderr: String::new(),
            passed: Some(0),
            failed: Some(1),
            skipped: None,
            duration_ms: 100,
        };
        let failures = runner.extract_failures(&result);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].test_name, "my_test");
    }

    #[test]
    fn has_failures_none_failed_success_true() {
        let result = TestResult {
            framework: TestFramework::Generic,
            success: true,
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            passed: None,
            failed: None,
            skipped: None,
            duration_ms: 50,
        };
        assert!(!result.has_failures());
    }

    #[tokio::test]
    async fn run_with_command_empty() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let err = runner.run_with_command(&[]).await.unwrap_err();
        assert!(matches!(err, TestError::CommandFailed(_)));
    }

    #[tokio::test]
    async fn run_tests_no_framework() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let mut runner = TestRunner::new(ctx);
        let err = runner.run_tests().await.unwrap_err();
        assert!(matches!(err, TestError::NoFrameworkDetected));
    }

    #[tokio::test]
    async fn run_specific_no_framework() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let err = runner.run_specific("test_name").await.unwrap_err();
        assert!(matches!(err, TestError::NoFrameworkDetected));
    }

    #[tokio::test]
    async fn run_specific_unsupported_framework() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx).with_framework(TestFramework::PythonUnittest);
        let err = runner.run_specific("test_name").await.unwrap_err();
        assert!(matches!(err, TestError::CommandFailed(_)));
    }

    #[tokio::test]
    async fn run_specific_generic_unsupported() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx).with_framework(TestFramework::Generic);
        let err = runner.run_specific("test_name").await.unwrap_err();
        assert!(matches!(err, TestError::CommandFailed(_)));
    }

    #[tokio::test]
    async fn run_with_command_nonexistent_binary() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let err = runner.run_with_command(&["__nonexistent_binary_xyz__"]).await.unwrap_err();
        assert!(matches!(err, TestError::ExecutionError(_)));
    }

    #[test]
    fn parse_pytest_only_passed() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = "===== 8 passed in 2.0s =====";
        let (p, f, s) = runner.parse_pytest_output(stdout);
        assert_eq!(p, Some(8));
        assert_eq!(f, None);
        assert_eq!(s, None);
    }

    #[test]
    fn parse_pytest_no_match() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let (p, f, s) = runner.parse_pytest_output("no relevant output");
        assert!(p.is_none());
        assert!(f.is_none());
        assert!(s.is_none());
    }

    #[test]
    fn parse_cargo_failures_no_panic_line() {
        let ctx = ExecutionContext::new("/tmp".into());
        let runner = TestRunner::new(ctx);
        let stdout = "---- my_test stdout ----\nsome other output\n----\n";
        let failures = runner.parse_cargo_failures(stdout);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].test_name, "my_test");
        assert!(failures[0].message.is_empty());
    }

    #[test]
    fn test_error_display() {
        assert_eq!(TestError::NoFrameworkDetected.to_string(), "no test framework detected");
        assert!(TestError::CommandFailed("x".into()).to_string().contains("x"));
        assert!(TestError::ParseError("y".into()).to_string().contains("y"));
    }
}
