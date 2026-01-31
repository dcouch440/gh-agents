//! Decomposition examples demonstrating good vertical slicing.

/// An example for training/demonstrating to agents.
#[derive(Debug, Clone)]
pub struct Example {
    pub title: String,
    pub domain: String,
    pub keywords: Vec<&'static str>,
    pub input: String,
    pub thinking: String,
    pub output: String,
    pub explanation: String,
}

impl Example {
    /// Format this example for inclusion in a prompt
    pub fn format_for_prompt(&self) -> String {
        format!(
            r#"### Example: {}

**Input:**
{}

**Thinking:**
{}

**Output:**
```json
{}
```

**Why this is good:** {}"#,
            self.title, self.input, self.thinking, self.output, self.explanation
        )
    }
}

/// Examples demonstrating good ticket decomposition.
pub struct DecompositionExamples;

impl DecompositionExamples {
    /// Get all decomposition examples
    pub fn all() -> Vec<Example> {
        vec![
            Self::user_auth_example(),
            Self::api_endpoint_example(),
            Self::database_migration_example(),
            Self::ui_component_example(),
            Self::refactoring_example(),
        ]
    }

    /// Get examples relevant to a task description
    pub fn for_task(task_description: &str) -> Vec<Example> {
        let lower = task_description.to_lowercase();
        let mut relevant = Vec::new();

        if lower.contains("auth") || lower.contains("login") || lower.contains("user") {
            relevant.push(Self::user_auth_example());
        }
        if lower.contains("api") || lower.contains("endpoint") || lower.contains("route") {
            relevant.push(Self::api_endpoint_example());
        }
        if lower.contains("database") || lower.contains("migration") || lower.contains("table") {
            relevant.push(Self::database_migration_example());
        }
        if lower.contains("ui") || lower.contains("component") || lower.contains("view") {
            relevant.push(Self::ui_component_example());
        }
        if lower.contains("refactor") || lower.contains("clean") || lower.contains("improve") {
            relevant.push(Self::refactoring_example());
        }

        // If nothing specific matched, return the auth example as it's comprehensive
        if relevant.is_empty() {
            relevant.push(Self::user_auth_example());
        }

        // Limit to 2 examples to save context
        relevant.truncate(2);
        relevant
    }

    fn user_auth_example() -> Example {
        Example {
            title: "User Authentication".to_string(),
            domain: "backend".to_string(),
            keywords: vec!["auth", "login", "user", "session", "jwt"],
            input: r#"Add user authentication to the API"#.to_string(),
            thinking: r#"I need to think about what "authentication" really means here and what vertical slices make sense.

UNDERSTAND: The user wants to protect API endpoints so only logged-in users can access them. Business value is security and personalization.

INVENTORY: Components touched:
- Database: users table with password storage
- API routes: login, logout, register endpoints
- Middleware: auth validation on protected routes
- Tests: for each component

DEPENDENCIES: Users must exist before we can authenticate them. Passwords must be hashed before we store them. Login must work before we can add middleware.

SLICE VERTICALLY: Each slice should result in something testable and independently valuable.

SIZE CHECK: Each slice looks like 1-3 hours of work. Good size."#.to_string(),
            output: r#"{
  "slices": [
    {
      "title": "User model + migration",
      "description": "Create users table with id, email, password_hash, created_at. Add User struct with basic CRUD.",
      "tasks": [
        {"title": "Create users table migration", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Create User struct with derives", "tier": "utility", "estimated_complexity": "low"},
        {"title": "Implement insert_user and get_user_by_email", "tier": "worker", "estimated_complexity": "medium"}
      ],
      "dependencies": [],
      "acceptance_criteria": ["Can insert a user to DB", "Can query user by email", "Tests pass"]
    },
    {
      "title": "Password hashing",
      "description": "Add password hashing on registration, validation on login. Never store plaintext.",
      "tasks": [
        {"title": "Add argon2 password hashing", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Add verify_password function", "tier": "worker", "estimated_complexity": "low"}
      ],
      "dependencies": ["User model + migration"],
      "acceptance_criteria": ["Password stored as hash", "Can verify correct password", "Rejects wrong password"]
    },
    {
      "title": "Login endpoint",
      "description": "POST /login that validates credentials and returns JWT token.",
      "tasks": [
        {"title": "Create login route handler", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Implement JWT token generation", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Add login tests", "tier": "worker", "estimated_complexity": "low"}
      ],
      "dependencies": ["User model + migration", "Password hashing"],
      "acceptance_criteria": ["Returns 200 + token for valid credentials", "Returns 401 for invalid", "Token contains user_id"]
    },
    {
      "title": "Auth middleware",
      "description": "Middleware that validates JWT and injects user into request context.",
      "tasks": [
        {"title": "Create auth middleware", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Add user to request extensions", "tier": "worker", "estimated_complexity": "low"}
      ],
      "dependencies": ["Login endpoint"],
      "acceptance_criteria": ["Valid token passes through", "Invalid token returns 401", "User available in handler"]
    },
    {
      "title": "Protected route example",
      "description": "One route that requires authentication to prove the flow works.",
      "tasks": [
        {"title": "Create GET /me endpoint", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Add integration test for full flow", "tier": "worker", "estimated_complexity": "medium"}
      ],
      "dependencies": ["Auth middleware"],
      "acceptance_criteria": ["Can register, login, access /me", "Full flow test passes"]
    }
  ],
  "questions": [],
  "risks": ["JWT secret must be configured securely", "Password requirements not specified"]
}"#.to_string(),
            explanation: "Each slice is independently testable. We didn't create horizontal slices like 'write all tests' or 'write all DB code'. Each slice touches all layers it needs to work.".to_string(),
        }
    }

    fn api_endpoint_example() -> Example {
        Example {
            title: "CRUD API Endpoint".to_string(),
            domain: "backend".to_string(),
            keywords: vec!["api", "endpoint", "crud", "rest", "route"],
            input: r#"Add a CRUD API for managing blog posts"#.to_string(),
            thinking: r#"UNDERSTAND: User wants full create/read/update/delete operations for blog posts. Standard REST API pattern.

INVENTORY: Need Post model, database table, 5 endpoints (list, get, create, update, delete), tests.

DEPENDENCIES: Model must exist before endpoints. Get endpoint helps test others.

SLICE VERTICALLY: Each operation is a natural slice. Start with read, then create, then update/delete."#.to_string(),
            output: r#"{
  "slices": [
    {
      "title": "Post model + list endpoint",
      "description": "Create posts table and GET /posts endpoint that returns all posts.",
      "tasks": [
        {"title": "Create posts table migration", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Create Post struct", "tier": "utility", "estimated_complexity": "low"},
        {"title": "Implement GET /posts", "tier": "worker", "estimated_complexity": "medium"}
      ],
      "dependencies": [],
      "acceptance_criteria": ["Posts table exists", "GET /posts returns empty array", "GET /posts returns posts when they exist"]
    },
    {
      "title": "Create post endpoint",
      "description": "POST /posts that creates a new post.",
      "tasks": [
        {"title": "Implement POST /posts handler", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Add input validation", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Add create tests", "tier": "worker", "estimated_complexity": "low"}
      ],
      "dependencies": ["Post model + list endpoint"],
      "acceptance_criteria": ["Can create post via API", "Returns created post with ID", "Invalid input returns 400"]
    },
    {
      "title": "Get single post endpoint",
      "description": "GET /posts/:id that returns a specific post.",
      "tasks": [
        {"title": "Implement GET /posts/:id", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Handle not found", "tier": "worker", "estimated_complexity": "low"}
      ],
      "dependencies": ["Post model + list endpoint"],
      "acceptance_criteria": ["Returns post by ID", "Returns 404 for missing ID"]
    },
    {
      "title": "Update and delete endpoints",
      "description": "PUT /posts/:id and DELETE /posts/:id for modifying posts.",
      "tasks": [
        {"title": "Implement PUT /posts/:id", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Implement DELETE /posts/:id", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Add update/delete tests", "tier": "worker", "estimated_complexity": "medium"}
      ],
      "dependencies": ["Get single post endpoint"],
      "acceptance_criteria": ["Can update post", "Can delete post", "Operations return appropriate status codes"]
    }
  ],
  "questions": ["Should posts have authors (user association)?", "Any soft delete requirements?"],
  "risks": []
}"#.to_string(),
            explanation: "Started with read operations since they help verify the model works. Each slice can be deployed and tested independently.".to_string(),
        }
    }

    fn database_migration_example() -> Example {
        Example {
            title: "Database Schema Change".to_string(),
            domain: "database".to_string(),
            keywords: vec!["database", "migration", "schema", "table", "column"],
            input: r#"Add soft delete functionality to all entities"#.to_string(),
            thinking: r#"UNDERSTAND: Add deleted_at timestamp to track soft deletes instead of actually removing records.

INVENTORY: Multiple tables affected, queries need to filter deleted records, admin might need to see deleted items.

DEPENDENCIES: Migration must run before code changes. All affected tables should be updated together for consistency.

SLICE VERTICALLY: Do one table at a time to reduce risk. Start with least critical table."#.to_string(),
            output: r#"{
  "slices": [
    {
      "title": "Soft delete for posts table",
      "description": "Add deleted_at to posts, update queries to filter by default.",
      "tasks": [
        {"title": "Create migration adding deleted_at", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Update Post queries to filter deleted", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Add soft_delete and restore methods", "tier": "worker", "estimated_complexity": "low"}
      ],
      "dependencies": [],
      "acceptance_criteria": ["deleted_at column exists", "Queries exclude deleted by default", "Can soft delete and restore"]
    },
    {
      "title": "Soft delete for comments table",
      "description": "Same pattern applied to comments.",
      "tasks": [
        {"title": "Create migration adding deleted_at", "tier": "utility", "estimated_complexity": "low"},
        {"title": "Update Comment queries", "tier": "worker", "estimated_complexity": "medium"}
      ],
      "dependencies": ["Soft delete for posts table"],
      "acceptance_criteria": ["Comments support soft delete", "Cascade behavior correct"]
    },
    {
      "title": "Admin view of deleted items",
      "description": "Allow admins to see and restore deleted items.",
      "tasks": [
        {"title": "Add include_deleted query option", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Add admin restore endpoint", "tier": "worker", "estimated_complexity": "medium"}
      ],
      "dependencies": ["Soft delete for posts table", "Soft delete for comments table"],
      "acceptance_criteria": ["Admins can list deleted items", "Can restore from admin panel"]
    }
  ],
  "questions": ["Which tables should have soft delete?", "Should there be a permanent delete for old items?"],
  "risks": ["Existing queries might return deleted items if not updated", "Foreign key constraints might complicate deletes"]
}"#.to_string(),
            explanation: "Did one table at a time so we can verify the pattern works before applying everywhere. Saved admin features for last since they depend on the core functionality.".to_string(),
        }
    }

    fn ui_component_example() -> Example {
        Example {
            title: "UI Component".to_string(),
            domain: "frontend".to_string(),
            keywords: vec!["ui", "component", "view", "widget", "screen"],
            input: r#"Add a user settings page with profile editing"#.to_string(),
            thinking: r#"UNDERSTAND: User wants a page where they can view and edit their profile settings.

INVENTORY: Need route, page component, form components, API integration, validation.

DEPENDENCIES: API endpoints for getting/updating user must exist (or mock them).

SLICE VERTICALLY: Start with static display, then add editing, then validation."#.to_string(),
            output: r#"{
  "slices": [
    {
      "title": "Settings page skeleton",
      "description": "Create route and basic page layout with hardcoded data.",
      "tasks": [
        {"title": "Add /settings route", "tier": "utility", "estimated_complexity": "low"},
        {"title": "Create SettingsPage component", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Add basic layout with sections", "tier": "worker", "estimated_complexity": "low"}
      ],
      "dependencies": [],
      "acceptance_criteria": ["Route accessible", "Page renders with placeholder content"]
    },
    {
      "title": "Profile display",
      "description": "Fetch and display current user profile data.",
      "tasks": [
        {"title": "Add useProfile hook", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Display profile fields", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Add loading state", "tier": "worker", "estimated_complexity": "low"}
      ],
      "dependencies": ["Settings page skeleton"],
      "acceptance_criteria": ["Shows current profile data", "Loading spinner while fetching", "Handles fetch errors"]
    },
    {
      "title": "Profile editing",
      "description": "Add edit mode with form inputs and save functionality.",
      "tasks": [
        {"title": "Add edit mode toggle", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Create profile form", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Implement save with API call", "tier": "worker", "estimated_complexity": "medium"}
      ],
      "dependencies": ["Profile display"],
      "acceptance_criteria": ["Can toggle edit mode", "Form pre-filled with current values", "Save persists changes"]
    },
    {
      "title": "Form validation",
      "description": "Add client-side validation and error display.",
      "tasks": [
        {"title": "Add validation rules", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Display field errors", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Add validation tests", "tier": "worker", "estimated_complexity": "low"}
      ],
      "dependencies": ["Profile editing"],
      "acceptance_criteria": ["Invalid fields show errors", "Can't submit invalid form", "Errors clear when fixed"]
    }
  ],
  "questions": ["What fields are editable?", "Any image upload for avatar?"],
  "risks": ["API endpoint format unknown", "Might need optimistic updates for good UX"]
}"#.to_string(),
            explanation: "Started with static, then dynamic, then interactive. Each slice results in something visible and testable.".to_string(),
        }
    }

    fn refactoring_example() -> Example {
        Example {
            title: "Code Refactoring".to_string(),
            domain: "refactoring".to_string(),
            keywords: vec!["refactor", "clean", "improve", "restructure", "extract"],
            input: r#"Refactor the authentication module to be more testable"#.to_string(),
            thinking: r#"UNDERSTAND: Current auth code is hard to test, probably due to tight coupling. Goal is better test coverage.

INVENTORY: Need to identify coupling points, extract interfaces, add dependency injection.

DEPENDENCIES: Must maintain existing behavior while refactoring. Tests are critical.

SLICE VERTICALLY: Add tests first to lock behavior, then refactor in small steps."#.to_string(),
            output: r#"{
  "slices": [
    {
      "title": "Add characterization tests",
      "description": "Write tests that capture current behavior before any changes.",
      "tasks": [
        {"title": "Write tests for current login flow", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Write tests for token validation", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Document current behavior", "tier": "utility", "estimated_complexity": "low"}
      ],
      "dependencies": [],
      "acceptance_criteria": ["Tests pass with current code", "All major paths covered", "Behavior documented"]
    },
    {
      "title": "Extract token service interface",
      "description": "Create trait for token operations, allowing mock injection.",
      "tasks": [
        {"title": "Define TokenService trait", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Implement trait for current code", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Verify tests still pass", "tier": "utility", "estimated_complexity": "low"}
      ],
      "dependencies": ["Add characterization tests"],
      "acceptance_criteria": ["Trait defined with clear contract", "Current behavior unchanged", "All tests pass"]
    },
    {
      "title": "Extract user repository interface",
      "description": "Create trait for user data access, allowing mock injection.",
      "tasks": [
        {"title": "Define UserRepository trait", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Implement trait for current DB code", "tier": "worker", "estimated_complexity": "medium"},
        {"title": "Update auth to use trait", "tier": "worker", "estimated_complexity": "low"}
      ],
      "dependencies": ["Extract token service interface"],
      "acceptance_criteria": ["Repository trait defined", "Auth code uses trait", "Tests pass"]
    },
    {
      "title": "Add unit tests with mocks",
      "description": "Write fast unit tests using mock implementations.",
      "tasks": [
        {"title": "Create mock TokenService", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Create mock UserRepository", "tier": "worker", "estimated_complexity": "low"},
        {"title": "Write unit tests with mocks", "tier": "worker", "estimated_complexity": "medium"}
      ],
      "dependencies": ["Extract user repository interface"],
      "acceptance_criteria": ["Unit tests run fast (no DB)", "Edge cases covered", "Mocks are simple"]
    }
  ],
  "questions": ["Any external services to mock?", "Target test coverage percentage?"],
  "risks": ["Might discover undocumented behavior during testing", "Risk of changing behavior accidentally"]
}"#.to_string(),
            explanation: "Critical pattern: add tests FIRST to lock current behavior, then refactor with safety net. Each slice maintains working state.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_examples_have_content() {
        let examples = DecompositionExamples::all();
        assert_eq!(examples.len(), 5);

        for ex in &examples {
            assert!(!ex.title.is_empty());
            assert!(!ex.input.is_empty());
            assert!(!ex.thinking.is_empty());
            assert!(!ex.output.is_empty());
            assert!(!ex.explanation.is_empty());
            assert!(!ex.keywords.is_empty());
        }
    }

    #[test]
    fn for_task_finds_auth_example() {
        let examples = DecompositionExamples::for_task("Add user authentication");
        assert!(!examples.is_empty());
        assert!(examples.iter().any(|e| e.title.contains("Authentication")));
    }

    #[test]
    fn for_task_finds_api_example() {
        let examples = DecompositionExamples::for_task("Create REST API endpoint");
        assert!(!examples.is_empty());
        assert!(examples.iter().any(|e| e.title.contains("API")));
    }

    #[test]
    fn for_task_returns_default_for_unknown() {
        let examples = DecompositionExamples::for_task("Something completely unrelated");
        assert!(!examples.is_empty()); // Returns auth example as fallback
    }

    #[test]
    fn for_task_limits_to_two() {
        // This should match multiple examples
        let examples = DecompositionExamples::for_task("refactor the user auth api endpoint");
        assert!(examples.len() <= 2);
    }

    #[test]
    fn format_for_prompt_includes_sections() {
        let example = DecompositionExamples::user_auth_example();
        let formatted = example.format_for_prompt();

        assert!(formatted.contains("### Example:"));
        assert!(formatted.contains("**Input:**"));
        assert!(formatted.contains("**Thinking:**"));
        assert!(formatted.contains("**Output:**"));
        assert!(formatted.contains("**Why this is good:**"));
    }
}
