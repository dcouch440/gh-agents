#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use super::super::{aggregate_subtask_outputs, topo_sort_subtasks};
    use crate::server::hub::dag::StepOutput;
    use crate::types::Subtask;

    fn make_subtask(id: &str, depends_on: Vec<&str>) -> Subtask {
        Subtask {
            id: id.into(),
            task_name: format!("Task {}", id),
            agent_id: Uuid::new_v4(),
            tools: vec![],
            prompt_template: format!("Do {}", id),
            depends_on: depends_on.into_iter().map(|s| s.into()).collect(),
            input_mapping: std::collections::HashMap::new(),
            output_schema: None,
        }
    }

    fn make_subtask_output(raw: &str) -> StepOutput {
        StepOutput {
            variable_name: String::new(),
            structured_output: serde_json::from_str(raw).ok(),
            raw_output: raw.into(),
        }
    }

    #[test]
    fn topo_sort_subtasks_linear() {
        // A -> B -> C
        let subtasks = vec![
            make_subtask("a", vec![]),
            make_subtask("b", vec!["a"]),
            make_subtask("c", vec!["b"]),
        ];

        let layers = topo_sort_subtasks(&subtasks).unwrap();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].len(), 1);
        assert_eq!(layers[0][0].id, "a");
        assert_eq!(layers[1][0].id, "b");
        assert_eq!(layers[2][0].id, "c");
    }

    #[test]
    fn topo_sort_subtasks_parallel() {
        // A and B independent, C depends on both
        let subtasks = vec![
            make_subtask("a", vec![]),
            make_subtask("b", vec![]),
            make_subtask("c", vec!["a", "b"]),
        ];

        let layers = topo_sort_subtasks(&subtasks).unwrap();
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].len(), 2); // a and b in parallel
        let first_layer_ids: Vec<&str> = layers[0].iter().map(|s| s.id.as_str()).collect();
        assert!(first_layer_ids.contains(&"a"));
        assert!(first_layer_ids.contains(&"b"));
        assert_eq!(layers[1].len(), 1);
        assert_eq!(layers[1][0].id, "c");
    }

    #[test]
    fn topo_sort_subtasks_cycle_detected() {
        // A depends on B, B depends on A
        let subtasks = vec![make_subtask("a", vec!["b"]), make_subtask("b", vec!["a"])];

        assert!(topo_sort_subtasks(&subtasks).is_err());
    }

    #[test]
    fn topo_sort_subtasks_unknown_dep() {
        let subtasks = vec![make_subtask("a", vec!["nonexistent"])];

        assert!(topo_sort_subtasks(&subtasks).is_err());
    }

    #[test]
    fn topo_sort_subtasks_empty() {
        let layers = topo_sort_subtasks(&[]).unwrap();
        assert!(layers.is_empty());
    }

    #[test]
    fn aggregate_all_outputs_mode() {
        let mut results = HashMap::new();
        results.insert(
            "db_schema".into(),
            make_subtask_output(r#"{"tables": ["users", "posts"]}"#),
        );
        results.insert(
            "api".into(),
            make_subtask_output(r#"{"endpoints": ["/users", "/posts"]}"#),
        );

        let order = vec!["db_schema".into(), "api".into()];
        let aggregated = aggregate_subtask_outputs(&results, "all_outputs", &order);

        assert!(aggregated.is_object());
        let obj = aggregated.as_object().unwrap();
        assert!(obj.contains_key("db_schema"));
        assert!(obj.contains_key("api"));
        assert_eq!(obj["db_schema"]["tables"][0].as_str().unwrap(), "users");
    }

    #[test]
    fn aggregate_final_output_mode() {
        let mut results = HashMap::new();
        results.insert("first".into(), make_subtask_output(r#"{"step": 1}"#));
        results.insert("last".into(), make_subtask_output(r#"{"step": 2}"#));

        let order = vec!["first".into(), "last".into()];
        let aggregated = aggregate_subtask_outputs(&results, "final_output", &order);

        assert_eq!(aggregated["step"].as_i64().unwrap(), 2);
    }

    #[test]
    fn aggregate_merge_mode() {
        let mut results = HashMap::new();
        results.insert(
            "a".into(),
            make_subtask_output(r#"{"color": "red", "size": 10}"#),
        );
        results.insert(
            "b".into(),
            make_subtask_output(r#"{"shape": "circle", "size": 20}"#),
        );

        let order = vec!["a".into(), "b".into()];
        let aggregated = aggregate_subtask_outputs(&results, "merge", &order);

        let obj = aggregated.as_object().unwrap();
        assert_eq!(obj["color"].as_str().unwrap(), "red");
        assert_eq!(obj["shape"].as_str().unwrap(), "circle");
        // Later value wins for "size"
        assert_eq!(obj["size"].as_i64().unwrap(), 20);
    }

    #[test]
    fn aggregate_final_output_skips_missing() {
        let mut results = HashMap::new();
        results.insert("first".into(), make_subtask_output(r#"{"data": "ok"}"#));
        // "second" is missing (simulating a failed subtask)

        let order = vec!["first".into(), "second".into()];
        let aggregated = aggregate_subtask_outputs(&results, "final_output", &order);

        // Falls back to "first" since "second" is missing
        assert_eq!(aggregated["data"].as_str().unwrap(), "ok");
    }
}
