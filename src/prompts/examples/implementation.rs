//! Implementation examples demonstrating the plan-then-code pattern.

use super::Example;

/// Examples demonstrating good implementation patterns.
pub struct ImplementationExamples;

impl ImplementationExamples {
    /// Get all implementation examples
    pub fn all() -> Vec<Example> {
        vec![
            Self::simple_function_example(),
            Self::api_handler_example(),
            Self::context_request_example(),
            Self::self_check_example(),
        ]
    }

    fn simple_function_example() -> Example {
        Example {
            title: "Simple Function Implementation".to_string(),
            domain: "utility".to_string(),
            keywords: vec!["function", "simple", "utility"],
            input: r#"Implement a function that validates email addresses"#.to_string(),
            thinking: r#"READ: Need to validate email addresses. Should return bool or Result.

IDENTIFY: Need to create new file or add to existing validation module.

REQUEST: Would like to see if there's an existing validation module.

PLAN: Use regex pattern for basic email validation. Return bool for simplicity."#
                .to_string(),
            output: r#"{
  "phase": "complete",
  "thinking": "Simple validation function using regex. Standard pattern covers most cases.",
  "plan": {
    "approach": "Use regex to validate email format",
    "files_to_modify": ["src/validation.rs"],
    "files_to_create": [],
    "estimated_complexity": "low"
  },
  "code_changes": [
    {
      "file": "src/validation.rs",
      "action": "modify",
      "content": "use regex::Regex;\nuse once_cell::sync::Lazy;\n\nstatic EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {\n    Regex::new(r\"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$\").unwrap()\n});\n\npub fn is_valid_email(email: &str) -> bool {\n    EMAIL_REGEX.is_match(email)\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn valid_emails() {\n        assert!(is_valid_email(\"test@example.com\"));\n        assert!(is_valid_email(\"user.name@domain.co.uk\"));\n    }\n\n    #[test]\n    fn invalid_emails() {\n        assert!(!is_valid_email(\"not-an-email\"));\n        assert!(!is_valid_email(\"missing@domain\"));\n        assert!(!is_valid_email(\"@no-local.com\"));\n    }\n}",
      "explanation": "Added email validation with lazy regex compilation for performance"
    }
  ],
  "verification": {
    "requirements_met": [
      {"requirement": "Validate email addresses", "evidence": "is_valid_email function checks format"}
    ],
    "tests_added": ["valid_emails test", "invalid_emails test"],
    "potential_issues": ["Regex doesn't catch all edge cases per RFC 5322"]
  },
  "status": "ready_for_review"
}"#
            .to_string(),
            explanation:
                "Clear plan, incremental implementation, tests included, honest about limitations."
                    .to_string(),
        }
    }

    fn api_handler_example() -> Example {
        Example {
            title: "API Handler Implementation".to_string(),
            domain: "backend".to_string(),
            keywords: vec!["api", "handler", "endpoint", "route"],
            input: r#"Add a POST /users endpoint that creates a new user"#.to_string(),
            thinking: r#"READ: Need POST /users endpoint to create users.

IDENTIFY: Need to modify routes file, add handler, update User operations.

REQUEST: Need to see current route structure and User type.

PLAN: Add route, create handler that validates input, creates user, returns created user."#
                .to_string(),
            output: r#"{
  "phase": "complete",
  "plan": {
    "approach": "Add route to router, create handler with validation, integrate with user service",
    "files_to_modify": ["src/routes/mod.rs", "src/routes/users.rs"],
    "files_to_create": [],
    "estimated_complexity": "medium"
  },
  "progress": {
    "current_step": "Completed implementation",
    "completed_steps": ["Added route", "Created handler", "Added validation", "Added tests"],
    "remaining_steps": []
  },
  "code_changes": [
    {
      "file": "src/routes/users.rs",
      "action": "modify",
      "content": "use axum::{Json, extract::State};\nuse crate::{AppState, models::User, error::AppError};\n\n#[derive(Deserialize)]\npub struct CreateUserRequest {\n    pub email: String,\n    pub name: String,\n}\n\npub async fn create_user(\n    State(state): State<AppState>,\n    Json(req): Json<CreateUserRequest>,\n) -> Result<Json<User>, AppError> {\n    // Validate email\n    if !is_valid_email(&req.email) {\n        return Err(AppError::BadRequest(\"Invalid email\".into()));\n    }\n    \n    // Check for existing user\n    if state.users.get_by_email(&req.email).await?.is_some() {\n        return Err(AppError::Conflict(\"Email already exists\".into()));\n    }\n    \n    // Create user\n    let user = state.users.create(&req.email, &req.name).await?;\n    Ok(Json(user))\n}",
      "explanation": "Handler validates input, checks for duplicates, creates user"
    }
  ],
  "verification": {
    "requirements_met": [
      {"requirement": "POST /users endpoint", "evidence": "Route added in routes/mod.rs"},
      {"requirement": "Creates new user", "evidence": "Handler calls users.create()"}
    ],
    "tests_added": ["test_create_user_success", "test_create_user_duplicate_email", "test_create_user_invalid_email"],
    "potential_issues": []
  },
  "status": "ready_for_review"
}"#
            .to_string(),
            explanation:
                "Showed clear progression through planning -> implementation -> verification."
                    .to_string(),
        }
    }

    fn context_request_example() -> Example {
        Example {
            title: "Context Request".to_string(),
            domain: "general".to_string(),
            keywords: vec!["context", "request", "information"],
            input: r#"Update the user service to support soft delete"#.to_string(),
            thinking: r#"READ: Need to add soft delete to user service.

IDENTIFY: Need to see current User struct and service implementation.

REQUEST: I don't have enough context. Need to see the user module."#
                .to_string(),
            output: r#"{
  "phase": "planning",
  "thinking": "I need to see the current implementation before I can add soft delete functionality.",
  "plan": null,
  "progress": {
    "current_step": "Gathering context",
    "completed_steps": ["Read task requirements"],
    "remaining_steps": ["Review current implementation", "Plan changes", "Implement"]
  },
  "code_changes": [],
  "context_requests": [
    "src/models/user.rs - Need to see User struct definition",
    "src/services/user_service.rs - Need to see current service methods"
  ],
  "status": "needs_context"
}"#
            .to_string(),
            explanation: "Correctly identified missing context and requested specific files rather than asking for everything.".to_string(),
        }
    }

    fn self_check_example() -> Example {
        Example {
            title: "Self-Check Before Submission".to_string(),
            domain: "general".to_string(),
            keywords: vec!["verification", "check", "review"],
            input: r#"[After implementation] Verify your work before submitting"#.to_string(),
            thinking: r#"Let me verify my work:

REQUIREMENTS: Task was to add pagination to list endpoint.
- Added page and limit query params
- Returns paginated results
- Didn't add total count header - should add that

CORRECTNESS: Code looks correct but I should add bounds checking for page/limit.

EDGE CASES:
- What if page=0? Should default to 1
- What if limit>100? Should cap it

TESTS:
- Happy path test
- Missing edge case tests

I found issues to fix before submitting."#
                .to_string(),
            output: r#"{
  "thinking": "Found issues during self-review that need fixing",
  "requirements_check": [
    {"requirement": "Add pagination params", "status": "met", "evidence": "page and limit in handler"},
    {"requirement": "Return paginated results", "status": "met", "evidence": "Using LIMIT/OFFSET in query"},
    {"requirement": "Include total count", "status": "not_met", "issue": "Forgot to add X-Total-Count header"}
  ],
  "bugs_found": [
    {"description": "No bounds checking on page/limit", "location": "list_users handler", "fix_applied": true, "fix_description": "Added min/max bounds"},
    {"description": "page=0 causes empty results", "location": "list_users handler", "fix_applied": true, "fix_description": "Default page to 1"}
  ],
  "test_coverage": {
    "happy_path_covered": true,
    "edge_cases_covered": ["page=0", "limit>100"],
    "missing_tests": ["empty results", "exactly one page of results"]
  },
  "confidence": "medium",
  "ready_to_submit": false,
  "blocking_issues": ["Need to add X-Total-Count header"]
}"#
            .to_string(),
            explanation: "Caught own mistakes, fixed what could be fixed, honestly reported remaining issues.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_examples_have_content() {
        let examples = ImplementationExamples::all();
        assert_eq!(examples.len(), 4);

        for ex in &examples {
            assert!(!ex.title.is_empty());
            assert!(!ex.input.is_empty());
            assert!(!ex.output.is_empty());
        }
    }

    #[test]
    fn has_context_request_example() {
        let examples = ImplementationExamples::all();
        assert!(examples.iter().any(|e| e.title.contains("Context")));
    }

    #[test]
    fn has_self_check_example() {
        let examples = ImplementationExamples::all();
        assert!(examples.iter().any(|e| e.title.contains("Self-Check")));
    }
}
