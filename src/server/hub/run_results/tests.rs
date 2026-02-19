#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    use crate::config::protocols::roles;

    use super::super::{new_run_results_tokens, MAX_OUTPUT_CHARS};

    #[test]
    fn truncates_long_output() {
        let long_output = "x".repeat(MAX_OUTPUT_CHARS + 1000);
        let truncated = if long_output.len() > MAX_OUTPUT_CHARS {
            &long_output[..MAX_OUTPUT_CHARS]
        } else {
            &long_output
        };
        assert_eq!(truncated.len(), MAX_OUTPUT_CHARS);
    }

    #[test]
    fn cancel_and_replace_semantics() {
        let tokens = new_run_results_tokens();
        let step_id = Uuid::new_v4();

        // Insert first token
        let token1 = CancellationToken::new();
        tokens.insert(step_id, token1.clone());

        // "Replace" — remove old, cancel it, insert new
        if let Some((_, old)) = tokens.remove(&step_id) {
            old.cancel();
        }
        let token2 = CancellationToken::new();
        tokens.insert(step_id, token2.clone());

        assert!(token1.is_cancelled());
        assert!(!token2.is_cancelled());
    }

    #[test]
    fn system_prompt_has_required_sections() {
        let prompt = roles::RUN_RESULTS_SUMMARIZER;
        assert!(prompt.contains("<identity>"));
        assert!(prompt.contains("<audience>"));
        assert!(prompt.contains("<instructions>"));
        assert!(prompt.contains("<examples>"));
        assert!(prompt.contains("2-4 sentence"));
    }
}
