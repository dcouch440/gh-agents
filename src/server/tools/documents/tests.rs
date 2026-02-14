#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::server::tools::documents::{execute_submit_ticket, title_to_ref_tag};

    // =========================================================================
    // title_to_ref_tag
    // =========================================================================

    #[test]
    fn title_to_ref_tag_basic() {
        assert_eq!(title_to_ref_tag("My Cool Document"), "my-cool-document");
    }

    #[test]
    fn title_to_ref_tag_special_chars() {
        assert_eq!(
            title_to_ref_tag("API Design (v2) — Draft!"),
            "api-design-v2--draft"
        );
    }

    #[test]
    fn title_to_ref_tag_empty() {
        assert_eq!(title_to_ref_tag(""), "");
    }

    #[test]
    fn title_to_ref_tag_single_word() {
        assert_eq!(title_to_ref_tag("README"), "readme");
    }

    // =========================================================================
    // execute_submit_ticket
    // =========================================================================

    #[tokio::test]
    async fn submit_ticket_valid() {
        let input = json!({
            "title": "Add auth",
            "description": "Implement JWT auth",
            "acceptance_criteria": ["Login works"],
            "files_to_modify": ["src/auth.rs"],
            "complexity": "M",
            "role": "worker"
        });

        let result = execute_submit_ticket(&input).await;

        assert_eq!(result["valid"], true);
        assert_eq!(result["ticket"]["title"], "Add auth");
        assert_eq!(result["ticket"]["complexity"], "M");
        assert_eq!(result["ticket"]["role"], "worker");
    }

    #[tokio::test]
    async fn submit_ticket_missing_title() {
        let input = json!({
            "description": "Something",
            "acceptance_criteria": ["Done"],
            "files_to_modify": ["src/main.rs"],
            "complexity": "S",
            "role": "worker"
        });

        let result = execute_submit_ticket(&input).await;

        assert_eq!(result["valid"], false);
        let errors: Vec<String> = result["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect();
        assert!(errors.iter().any(|e| e.contains("title")));
    }

    #[tokio::test]
    async fn submit_ticket_invalid_complexity() {
        let input = json!({
            "title": "Fix bug",
            "description": "Fix the bug",
            "acceptance_criteria": ["Bug fixed"],
            "files_to_modify": ["src/bug.rs"],
            "complexity": "XXL",
            "role": "worker"
        });

        let result = execute_submit_ticket(&input).await;

        assert_eq!(result["valid"], false);
        let errors: Vec<String> = result["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect();
        assert!(errors.iter().any(|e| e.contains("complexity")));
    }

    #[tokio::test]
    async fn submit_ticket_invalid_role() {
        let input = json!({
            "title": "Fix bug",
            "description": "Fix the bug",
            "acceptance_criteria": ["Bug fixed"],
            "files_to_modify": ["src/bug.rs"],
            "complexity": "S",
            "role": "admin"
        });

        let result = execute_submit_ticket(&input).await;

        assert_eq!(result["valid"], false);
        let errors: Vec<String> = result["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect();
        assert!(errors.iter().any(|e| e.contains("role")));
    }

    #[tokio::test]
    async fn submit_ticket_empty_arrays() {
        let input = json!({
            "title": "Fix bug",
            "description": "Fix the bug",
            "acceptance_criteria": [],
            "files_to_modify": [],
            "complexity": "S",
            "role": "worker"
        });

        let result = execute_submit_ticket(&input).await;

        assert_eq!(result["valid"], false);
        let errors: Vec<String> = result["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap().to_string())
            .collect();
        assert!(errors.iter().any(|e| e.contains("acceptance_criteria")));
        assert!(errors.iter().any(|e| e.contains("files_to_modify")));
    }

    #[tokio::test]
    async fn submit_ticket_with_dependencies() {
        let input = json!({
            "title": "Add auth",
            "description": "Implement JWT auth",
            "acceptance_criteria": ["Login works"],
            "files_to_modify": ["src/auth.rs"],
            "complexity": "M",
            "role": "worker",
            "dependencies": ["Setup DB", "Create user model"]
        });

        let result = execute_submit_ticket(&input).await;

        assert_eq!(result["valid"], true);
        let deps = result["ticket"]["dependencies"].as_array().unwrap();
        assert_eq!(deps.len(), 2);
    }
}
