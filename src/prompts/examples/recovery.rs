//! Recovery examples demonstrating failure handling and escalation.

use super::Example;

/// Examples demonstrating failure recovery patterns.
pub struct RecoveryExamples;

impl RecoveryExamples {
    /// Get all recovery examples
    pub fn all() -> Vec<Example> {
        vec![
            Self::successful_recovery_example(),
            Self::escalation_example(),
        ]
    }

    fn successful_recovery_example() -> Example {
        Example {
            title: "Successful Recovery".to_string(),
            domain: "recovery".to_string(),
            keywords: vec!["retry", "fix", "recover"],
            input: r#"Failure: JSON parse error - missing closing brace"#.to_string(),
            thinking: r#"ANALYZE: My JSON output was malformed - missing closing brace.

CAUSE: I truncated my output. Simple formatting mistake.

RECOVERABLE: Yes, just need to output valid JSON.

STRATEGY: Retry with correct JSON formatting.

CONFIDENCE: High - this is a simple fix."#
                .to_string(),
            output: r#"{
  "thinking": "Simple JSON formatting error. Will retry with valid output.",
  "root_cause": "Truncated JSON output, missing closing brace",
  "recoverable": true,
  "strategy": "retry",
  "adjustment_details": "Will output complete, valid JSON this time",
  "escalation_reason": null,
  "recommended_action": "Regenerate output with proper JSON structure",
  "confidence": "high"
}"#
            .to_string(),
            explanation:
                "Simple errors should be retried immediately. High confidence = no need to escalate."
                    .to_string(),
        }
    }

    fn escalation_example() -> Example {
        Example {
            title: "Knowing When to Escalate".to_string(),
            domain: "recovery".to_string(),
            keywords: vec!["escalate", "stuck", "help"],
            input: r#"Failure: Third attempt at fixing compile error, still failing"#.to_string(),
            thinking: r#"ANALYZE: Same compile error three times. I've tried different approaches but the error persists.

CAUSE: I don't fully understand how the type system works here. My fixes are guesses.

RECOVERABLE: Not by me - I'm making the same mistake repeatedly.

STRATEGY: Escalate - need someone who understands this better.

CONFIDENCE: Low in my ability to fix this."#
                .to_string(),
            output: r#"{
  "thinking": "After 3 attempts I'm not making progress. Need help.",
  "root_cause": "Don't fully understand the type system interaction",
  "recoverable": false,
  "strategy": "escalate",
  "adjustment_details": null,
  "escalation_reason": "3 failed attempts with same error. My approaches aren't working.",
  "recommended_action": "Escalate to orchestrator or human for guidance",
  "confidence": "low"
}"#
            .to_string(),
            explanation: "After 3 failed attempts, escalate rather than keep guessing. Honest about limitations.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_examples_have_content() {
        let examples = RecoveryExamples::all();
        assert_eq!(examples.len(), 2);

        for ex in &examples {
            assert!(!ex.title.is_empty());
            assert!(!ex.input.is_empty());
            assert!(!ex.output.is_empty());
        }
    }

    #[test]
    fn has_successful_recovery() {
        let examples = RecoveryExamples::all();
        assert!(examples.iter().any(|e| e.title.contains("Successful")));
    }

    #[test]
    fn has_escalation_example() {
        let examples = RecoveryExamples::all();
        assert!(examples.iter().any(|e| e.title.contains("Escalat")));
    }

    #[test]
    fn successful_recovery_is_recoverable() {
        let examples = RecoveryExamples::all();
        let recovery = examples
            .iter()
            .find(|e| e.title.contains("Successful"))
            .unwrap();

        assert!(recovery.output.contains("\"recoverable\": true"));
    }

    #[test]
    fn escalation_is_not_recoverable() {
        let examples = RecoveryExamples::all();
        let escalation = examples
            .iter()
            .find(|e| e.title.contains("Escalat"))
            .unwrap();

        assert!(escalation.output.contains("\"recoverable\": false"));
    }
}
