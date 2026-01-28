//! Decomposition regression tests.

mod prompts;

use prompts::assertions::*;
use prompts::harness::PromptTestHarness;

const MOCK_DECOMPOSITION_RESPONSE: &str = r#"{
  "thinking": "I need to break down user authentication into vertical slices. This involves user model, password handling, login endpoint, middleware, and integration.",
  "slices": [
    {
      "title": "User model + migration",
      "description": "Create users table and User struct",
      "tasks": [
        {"title": "Create migration", "tier": "worker", "estimated_complexity": "low", "context_files": ["src/db/mod.rs"]}
      ],
      "dependencies": [],
      "acceptance_criteria": ["Users table exists", "Can insert user"]
    },
    {
      "title": "Password hashing",
      "description": "Add argon2 hashing",
      "tasks": [
        {"title": "Add hashing functions", "tier": "worker", "estimated_complexity": "medium", "context_files": ["src/auth/mod.rs"]}
      ],
      "dependencies": ["User model + migration"],
      "acceptance_criteria": ["Passwords stored hashed", "Can verify passwords"]
    }
  ],
  "questions": [],
  "risks": ["JWT secret configuration needed"]
}"#;

fn hash_fixture_prompt(harness: &PromptTestHarness, fixture_name: &str) -> String {
    // Load the fixture and create the prompt the same way the harness does
    let fixture = harness.load_fixture(fixture_name).unwrap();
    let prompt = format!(
        "Decompose this ticket:\n\n{}\n\n{}",
        fixture.input,
        fixture.additional_context.unwrap_or_default()
    );
    harness.hash_prompt(&prompt)
}

#[tokio::test]
async fn test_decomposition_produces_valid_output() {
    let harness = PromptTestHarness::new("tests/prompts/fixtures/tickets");
    let hash = hash_fixture_prompt(&harness, "auth");
    let harness = harness.mock_response(&hash, MOCK_DECOMPOSITION_RESPONSE);

    let result = harness.test_decomposition("auth").await.unwrap();

    let mut suite = AssertionSuite::new();
    suite.add("schema", PromptAssertions::output_matches_schema(&result));
    suite.add("reasoning", PromptAssertions::contains_reasoning(&result));
    suite.add("vertical", PromptAssertions::slices_are_vertical(&result));
    suite.add(
        "criteria",
        PromptAssertions::all_slices_have_acceptance_criteria(&result),
    );

    assert!(suite.all_passed(), "Failures:\n{}", suite.detailed_report());
}

#[tokio::test]
async fn test_decomposition_minimum_slices() {
    let harness = PromptTestHarness::new("tests/prompts/fixtures/tickets");
    let hash = hash_fixture_prompt(&harness, "auth");
    let harness = harness.mock_response(&hash, MOCK_DECOMPOSITION_RESPONSE);

    let result = harness.test_decomposition("auth").await.unwrap();

    let assertion = PromptAssertions::minimum_slices(&result, 2);
    assert!(
        assertion.is_pass(),
        "Expected at least 2 slices: {}",
        assertion.message()
    );
}

#[tokio::test]
async fn test_decomposition_has_valid_dependencies() {
    let harness = PromptTestHarness::new("tests/prompts/fixtures/tickets");
    let hash = hash_fixture_prompt(&harness, "auth");
    let harness = harness.mock_response(&hash, MOCK_DECOMPOSITION_RESPONSE);

    let result = harness.test_decomposition("auth").await.unwrap();

    let assertion = PromptAssertions::dependencies_are_valid(&result);
    assert!(
        assertion.is_pass(),
        "Dependencies should be valid: {}",
        assertion.message()
    );
}

#[tokio::test]
async fn test_decomposition_all_slices_have_tasks() {
    let harness = PromptTestHarness::new("tests/prompts/fixtures/tickets");
    let hash = hash_fixture_prompt(&harness, "auth");
    let harness = harness.mock_response(&hash, MOCK_DECOMPOSITION_RESPONSE);

    let result = harness.test_decomposition("auth").await.unwrap();

    let assertion = PromptAssertions::all_slices_have_tasks(&result);
    assert!(
        assertion.is_pass(),
        "All slices should have tasks: {}",
        assertion.message()
    );
}

#[tokio::test]
async fn test_decomposition_no_hallucinated_files() {
    let harness = PromptTestHarness::new("tests/prompts/fixtures/tickets");
    let hash = hash_fixture_prompt(&harness, "auth");
    let harness = harness.mock_response(&hash, MOCK_DECOMPOSITION_RESPONSE);

    let result = harness.test_decomposition("auth").await.unwrap();

    // These are files that we know exist in the response
    let known_files = vec!["src/db/mod.rs", "src/auth/mod.rs"];
    let assertion = PromptAssertions::no_hallucinated_files(&result, &known_files);
    assert!(
        assertion.is_pass(),
        "Should not have hallucinated files: {}",
        assertion.message()
    );
}

#[tokio::test]
async fn test_decomposition_assertion_suite_comprehensive() {
    let harness = PromptTestHarness::new("tests/prompts/fixtures/tickets");
    let hash = hash_fixture_prompt(&harness, "auth");
    let harness = harness.mock_response(&hash, MOCK_DECOMPOSITION_RESPONSE);

    let result = harness.test_decomposition("auth").await.unwrap();

    let mut suite = AssertionSuite::new();

    // Add all available assertions
    suite.add("schema", PromptAssertions::output_matches_schema(&result));
    suite.add("reasoning", PromptAssertions::contains_reasoning(&result));
    suite.add("vertical", PromptAssertions::slices_are_vertical(&result));
    suite.add(
        "criteria",
        PromptAssertions::all_slices_have_acceptance_criteria(&result),
    );
    suite.add("tasks", PromptAssertions::all_slices_have_tasks(&result));
    suite.add(
        "dependencies",
        PromptAssertions::dependencies_are_valid(&result),
    );
    suite.add("min_slices", PromptAssertions::minimum_slices(&result, 1));

    println!("Assertion Report:\n{}", suite.detailed_report());

    assert!(
        suite.all_passed(),
        "Some assertions failed:\n{}",
        suite.detailed_report()
    );
}
