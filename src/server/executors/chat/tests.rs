#[cfg(test)]
mod tests {
    //! Tests for chat consumer

    use super::super::*;
    use crate::db::traits::ChatMessageRepo;
    use crate::db::ChatMessageRow;
    use crate::server::state::test_helpers::MockReposBuilder;
    use crate::server::state::AppStateBuilder;
    use crate::types::{AppConfig, UserId};
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;

    /// Minimal in-memory chat message repo for orchestrator tests
    struct TestChatMessageRepo {
        messages: std::sync::Mutex<Vec<ChatMessageRow>>,
    }

    impl TestChatMessageRepo {
        fn new() -> Self {
            Self {
                messages: std::sync::Mutex::new(vec![]),
            }
        }
    }

    #[async_trait::async_trait]
    impl ChatMessageRepo for TestChatMessageRepo {
        async fn insert_chat_message(
            &self,
            _user_id: UserId,
            id: Uuid,
            role: String,
            content: String,
        ) -> anyhow::Result<()> {
            self.messages.lock().unwrap().push(ChatMessageRow {
                id,
                role,
                content,
                timestamp: Utc::now(),
                source_type: None,
                error: None,
            });
            Ok(())
        }
        async fn get_chat_history(
            &self,
            _user_id: UserId,
            limit: u32,
            offset: u32,
        ) -> anyhow::Result<Vec<ChatMessageRow>> {
            let msgs = self.messages.lock().unwrap();
            Ok(msgs
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect())
        }
        async fn set_chat_message_error(&self, id: Uuid, error: String) -> anyhow::Result<()> {
            let mut msgs = self.messages.lock().unwrap();
            if let Some(m) = msgs.iter_mut().find(|m| m.id == id) {
                m.error = Some(error);
            }
            Ok(())
        }
        async fn clear_chat_history(&self, _user_id: UserId) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn chat_consumer_sends_error_when_no_api_key() {
        let saved = std::env::var(crate::constants::ENV_ANTHROPIC_API_KEY).ok();
        std::env::remove_var(crate::constants::ENV_ANTHROPIC_API_KEY);

        let chat_repo: Arc<dyn ChatMessageRepo> = Arc::new(TestChatMessageRepo::new());
        let repos = MockReposBuilder::new()
            .with_chat_messages(chat_repo)
            .build();

        let (state, chat_rx) = AppStateBuilder::new()
            .with_repos(repos)
            .with_config(AppConfig::default())
            .build_for_test();

        let msg_id = Uuid::new_v4();
        let (_buf, mut rx, _done) = state.get_response_stream(msg_id);

        state
            .chat_tx()
            .send(ConsumerMessage {
                id: msg_id,
                user_id: UserId::new(),
                session_id: None,
                agent_id: None,
                content: "Hello".into(),
                timestamp: Utc::now(),
            })
            .await
            .unwrap();

        let _handle = spawn_chat_consumer(state, chat_rx);

        let chunk = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("timeout waiting for chunk")
            .expect("channel closed");

        assert!(matches!(chunk, StreamChunk::Error(_)));

        if let Some(key) = saved {
            std::env::set_var(crate::constants::ENV_ANTHROPIC_API_KEY, key);
        }
    }
}
