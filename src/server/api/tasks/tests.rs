//! Tests for task management endpoints

#[cfg(test)]
mod tests {
    use crate::server::api::tasks::*;

    #[test]
    fn create_task_request_deserializes() {
        let json = r#"{"title": "Test task", "priority": "high"}"#;
        let request: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "Test task");
        assert_eq!(request.priority, Some("high".to_string()));
    }

    #[test]
    fn tasks_query_deserializes() {
        let json = r#"{"status": "pending", "limit": 10}"#;
        let query: TasksQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.status, Some("pending".to_string()));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn create_task_request_all_fields() {
        let json = r#"{"title":"T","description":"D","priority":"low","tier":"orchestrator"}"#;
        let request: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "T");
        assert_eq!(request.description, Some("D".to_string()));
        assert_eq!(request.priority, Some("low".to_string()));
        assert_eq!(request.tier, Some("orchestrator".to_string()));
    }

    #[test]
    fn create_task_request_minimal() {
        let json = r#"{"title":"T"}"#;
        let request: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "T");
        assert!(request.description.is_none());
        assert!(request.priority.is_none());
        assert!(request.tier.is_none());
    }

    #[test]
    fn tasks_query_with_no_fields() {
        let json = r#"{}"#;
        let query: TasksQuery = serde_json::from_str(json).unwrap();
        assert!(query.status.is_none());
        assert!(query.limit.is_none());
    }
}
