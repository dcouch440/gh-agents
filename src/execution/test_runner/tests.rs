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
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"devDependencies":{"jest":"^29"}}"#,
    )
    .unwrap();

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

    assert_eq!(
        result.summary(),
        "PASSED: 10 passed, 0 failed, 2 skipped (1500 ms)"
    );
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
    assert_eq!(
        TestFramework::Cargo.default_command(),
        vec!["cargo", "test"]
    );
    assert_eq!(TestFramework::Npm.default_command(), vec!["npm", "test"]);
    assert_eq!(TestFramework::Jest.default_command(), vec!["npx", "jest"]);
    assert_eq!(
        TestFramework::Pytest.default_command(),
        vec!["pytest", "-v"]
    );
    assert_eq!(
        TestFramework::PythonUnittest.default_command(),
        vec!["python", "-m", "unittest", "discover"]
    );
    assert_eq!(
        TestFramework::Go.default_command(),
        vec!["go", "test", "./..."]
    );
    assert_eq!(
        TestFramework::Generic.default_command(),
        vec!["make", "test"]
    );
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
    std::fs::write(
        tmp.path().join("pyproject.toml"),
        "[tool.pytest]\nminversion = \"6.0\"",
    )
    .unwrap();
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
    std::fs::write(
        tmp.path().join("setup.py"),
        "from setuptools import setup\nsetup()",
    )
    .unwrap();
    let ctx = ExecutionContext::new(tmp.path().to_path_buf());
    let mut runner = TestRunner::new(ctx);
    assert_eq!(
        runner.detect_framework(),
        Some(TestFramework::PythonUnittest)
    );
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
    std::fs::write(
        tmp.path().join("requirements.txt"),
        "pytest==7.0\nrequests\n",
    )
    .unwrap();

    let ctx = ExecutionContext::new(tmp.path().to_path_buf());
    let mut runner = TestRunner::new(ctx);
    assert_eq!(runner.detect_framework(), Some(TestFramework::Pytest));
}

#[test]
fn has_pytest_pyproject_with_pytest_keyword() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("pyproject.toml"),
        "[project]\ndependencies = [\"pytest\"]\n",
    )
    .unwrap();

    let ctx = ExecutionContext::new(tmp.path().to_path_buf());
    let mut runner = TestRunner::new(ctx);
    assert_eq!(runner.detect_framework(), Some(TestFramework::Pytest));
}

#[test]
fn has_pytest_returns_false_no_pytest_indicators() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("pyproject.toml"),
        "[build-system]\nrequires = [\"setuptools\"]\n",
    )
    .unwrap();

    let ctx = ExecutionContext::new(tmp.path().to_path_buf());
    let mut runner = TestRunner::new(ctx);
    assert_eq!(
        runner.detect_framework(),
        Some(TestFramework::PythonUnittest)
    );
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
    let err = runner
        .run_with_command(&["__nonexistent_binary_xyz__"])
        .await
        .unwrap_err();
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
    assert_eq!(
        TestError::NoFrameworkDetected.to_string(),
        "no test framework detected"
    );
    assert!(TestError::CommandFailed("x".into())
        .to_string()
        .contains("x"));
    assert!(TestError::ParseError("y".into()).to_string().contains("y"));
}
}
