//! Implementation regression tests.

mod prompts;

use prompts::assertions::AssertionResult;

const MOCK_IMPLEMENTATION_RESPONSE: &str = r#"{
  "phase": "complete",
  "thinking": "Simple validation function. Will use regex pattern.",
  "plan": {
    "approach": "Use regex for email validation",
    "files_to_modify": ["src/validation.rs"],
    "files_to_create": [],
    "estimated_complexity": "low"
  },
  "progress": {
    "current_step": "Done",
    "completed_steps": ["Implemented function", "Added tests"],
    "remaining_steps": []
  },
  "code_changes": [
    {
      "file": "src/validation.rs",
      "action": "modify",
      "content": "pub fn is_valid_email(email: &str) -> bool { ... }",
      "explanation": "Email validation function"
    }
  ],
  "verification": {
    "requirements_met": [
      {"requirement": "Validate emails", "evidence": "Function returns true/false"}
    ],
    "tests_added": ["test_valid_emails", "test_invalid_emails"],
    "potential_issues": []
  },
  "status": "ready_for_review"
}"#;

/// Assert that implementation has a plan
fn has_plan(response: &str) -> AssertionResult {
    if response.contains("\"plan\"") && response.contains("\"approach\"") {
        AssertionResult::pass("Has implementation plan")
    } else {
        AssertionResult::fail("Missing implementation plan")
    }
}

/// Assert that implementation has verification
fn has_verification(response: &str) -> AssertionResult {
    if response.contains("\"verification\"") && response.contains("\"requirements_met\"") {
        AssertionResult::pass("Has verification")
    } else {
        AssertionResult::fail("Missing verification section")
    }
}

/// Assert that code changes include explanations
fn code_changes_have_explanations(response: &str) -> AssertionResult {
    if response.contains("\"explanation\"") {
        AssertionResult::pass("Code changes have explanations")
    } else {
        AssertionResult::fail("Code changes missing explanations")
    }
}

/// Assert that thinking is present
fn has_thinking(response: &str) -> AssertionResult {
    if response.contains("\"thinking\"") {
        AssertionResult::pass("Has thinking")
    } else {
        AssertionResult::fail("Missing thinking section")
    }
}

/// Assert that progress tracking is present
fn has_progress(response: &str) -> AssertionResult {
    if response.contains("\"progress\"") && response.contains("\"current_step\"") {
        AssertionResult::pass("Has progress tracking")
    } else {
        AssertionResult::fail("Missing progress tracking")
    }
}

/// Assert that status is present
fn has_status(response: &str) -> AssertionResult {
    if response.contains("\"status\"") {
        AssertionResult::pass("Has status")
    } else {
        AssertionResult::fail("Missing status")
    }
}

/// Assert that code changes list files
fn code_changes_list_files(response: &str) -> AssertionResult {
    if response.contains("\"file\"") && response.contains("\"action\"") {
        AssertionResult::pass("Code changes list files and actions")
    } else {
        AssertionResult::fail("Code changes missing file or action")
    }
}

#[tokio::test]
async fn test_implementation_has_plan() {
    let assertion = has_plan(MOCK_IMPLEMENTATION_RESPONSE);
    assert!(assertion.is_pass(), "{}", assertion.message());
}

#[tokio::test]
async fn test_implementation_has_verification() {
    let assertion = has_verification(MOCK_IMPLEMENTATION_RESPONSE);
    assert!(assertion.is_pass(), "{}", assertion.message());
}

#[tokio::test]
async fn test_implementation_code_has_explanations() {
    let assertion = code_changes_have_explanations(MOCK_IMPLEMENTATION_RESPONSE);
    assert!(assertion.is_pass(), "{}", assertion.message());
}

#[tokio::test]
async fn test_implementation_has_thinking() {
    let assertion = has_thinking(MOCK_IMPLEMENTATION_RESPONSE);
    assert!(assertion.is_pass(), "{}", assertion.message());
}

#[tokio::test]
async fn test_implementation_has_progress() {
    let assertion = has_progress(MOCK_IMPLEMENTATION_RESPONSE);
    assert!(assertion.is_pass(), "{}", assertion.message());
}

#[tokio::test]
async fn test_implementation_has_status() {
    let assertion = has_status(MOCK_IMPLEMENTATION_RESPONSE);
    assert!(assertion.is_pass(), "{}", assertion.message());
}

#[tokio::test]
async fn test_implementation_code_changes_list_files() {
    let assertion = code_changes_list_files(MOCK_IMPLEMENTATION_RESPONSE);
    assert!(assertion.is_pass(), "{}", assertion.message());
}

#[tokio::test]
async fn test_implementation_all_assertions() {
    use prompts::assertions::AssertionSuite;

    let mut suite = AssertionSuite::new();
    suite.add("plan", has_plan(MOCK_IMPLEMENTATION_RESPONSE));
    suite.add(
        "verification",
        has_verification(MOCK_IMPLEMENTATION_RESPONSE),
    );
    suite.add(
        "explanations",
        code_changes_have_explanations(MOCK_IMPLEMENTATION_RESPONSE),
    );
    suite.add("thinking", has_thinking(MOCK_IMPLEMENTATION_RESPONSE));
    suite.add("progress", has_progress(MOCK_IMPLEMENTATION_RESPONSE));
    suite.add("status", has_status(MOCK_IMPLEMENTATION_RESPONSE));
    suite.add(
        "file_changes",
        code_changes_list_files(MOCK_IMPLEMENTATION_RESPONSE),
    );

    println!(
        "Implementation Assertion Report:\n{}",
        suite.detailed_report()
    );

    assert!(
        suite.all_passed(),
        "Some assertions failed:\n{}",
        suite.detailed_report()
    );
}

// Test with incomplete implementation response
const MOCK_INCOMPLETE_RESPONSE: &str = r#"{
  "phase": "implementing",
  "thinking": "Working on it...",
  "progress": {
    "current_step": "Adding tests",
    "completed_steps": ["Implemented function"],
    "remaining_steps": ["Fix edge cases"]
  },
  "status": "in_progress"
}"#;

#[tokio::test]
async fn test_incomplete_implementation_missing_plan() {
    let assertion = has_plan(MOCK_INCOMPLETE_RESPONSE);
    assert!(
        assertion.is_fail(),
        "Incomplete response should fail plan check"
    );
}

#[tokio::test]
async fn test_incomplete_implementation_has_progress() {
    let assertion = has_progress(MOCK_INCOMPLETE_RESPONSE);
    assert!(
        assertion.is_pass(),
        "Incomplete response should still have progress"
    );
}
