#[cfg(test)]
mod tests {
    //! Tests for hub error types

    use crate::llm::LLMError;
    use crate::server::hub::error::HubError;
    use uuid::Uuid;

    #[test]
    fn display_provider_not_configured() {
        let err = HubError::ProviderNotConfigured;
        assert_eq!(err.to_string(), "LLM provider not configured");
    }

    #[test]
    fn display_llm_call_failed() {
        let err = HubError::LlmCallFailed {
            round: 3,
            source: LLMError::Timeout(5000),
        };
        assert!(err.to_string().contains("round 3"));
    }

    #[test]
    fn display_context_budget() {
        let err = HubError::ContextBudgetExceeded {
            chars: 500_000,
            round: 7,
        };
        assert!(err.to_string().contains("500000"));
        assert!(err.to_string().contains("round 7"));
    }

    #[test]
    fn display_tool_failed() {
        let err = HubError::ToolFailed {
            tool_name: "search".into(),
            reason: "timeout".into(),
        };
        assert!(err.to_string().contains("search"));
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn display_max_rounds() {
        let err = HubError::MaxRoundsExhausted { max: 10 };
        assert!(err.to_string().contains("10"));
    }

    #[test]
    fn display_unknown_mode() {
        let err = HubError::UnknownMode {
            mode_id: "ghost".into(),
        };
        assert!(err.to_string().contains("ghost"));
    }

    #[test]
    fn display_dag_cycle() {
        let err = HubError::DagCycle;
        assert_eq!(err.to_string(), "workflow cycle detected");
    }

    #[test]
    fn display_unresolved_variable() {
        let err = HubError::UnresolvedVariable {
            path: "steps.0.output".into(),
        };
        assert!(err.to_string().contains("steps.0.output"));
    }

    #[test]
    fn display_agent_not_found() {
        let step = Uuid::nil();
        let agent = Uuid::nil();
        let err = HubError::AgentNotFound {
            step_id: step,
            agent_id: agent,
        };
        assert!(err.to_string().contains("00000000"));
    }

    #[test]
    fn display_awaiting_user() {
        let err = HubError::AwaitingUser {
            step_id: Uuid::nil(),
            execution_id: Uuid::nil(),
        };
        assert!(err.to_string().contains("awaiting user input"));
    }

    #[test]
    fn display_cancelled() {
        let err = HubError::Cancelled;
        assert_eq!(err.to_string(), "execution cancelled");
    }

    #[test]
    fn display_stream_interrupted() {
        let err = HubError::StreamInterrupted {
            execution_id: Uuid::nil(),
        };
        assert!(err.to_string().contains("interrupted"));
    }

    #[test]
    fn from_anyhow() {
        let inner = anyhow::anyhow!("something broke");
        let err: HubError = inner.into();
        assert!(err.to_string().contains("something broke"));
    }
}
