use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::SessionRepo;
use crate::db::{ChatMessageRow, SessionRow};
use crate::types::UserId;

use super::PgRepo;

#[async_trait]
impl SessionRepo for PgRepo {
    async fn create_session(
        &self,
        user_id: UserId,
        session_id: Uuid,
        mode_id: &str,
        title: &str,
        agent_id: Option<Uuid>,
        draft_config: Option<serde_json::Value>,
    ) -> Result<()> {
        crate::db::create_session(
            &self.pool,
            user_id,
            session_id,
            mode_id,
            title,
            agent_id,
            draft_config,
        )
        .await
    }

    async fn list_sessions(&self, user_id: UserId) -> Result<Vec<SessionRow>> {
        crate::db::list_sessions(&self.pool, user_id).await
    }

    async fn get_session(&self, session_id: Uuid) -> Result<Option<SessionRow>> {
        crate::db::get_session(&self.pool, session_id).await
    }

    async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        crate::db::delete_session(&self.pool, session_id).await
    }

    async fn insert_session_message(
        &self,
        user_id: UserId,
        session_id: Uuid,
        id: Uuid,
        role: String,
        content: String,
    ) -> Result<()> {
        crate::db::insert_session_message(&self.pool, user_id, session_id, &id, &role, &content)
            .await
    }

    async fn insert_agent_message(
        &self,
        user_id: UserId,
        session_id: Uuid,
        id: Uuid,
        role: String,
        content: String,
        source_type: String,
    ) -> Result<()> {
        crate::db::insert_agent_session_message(
            &self.pool,
            user_id,
            session_id,
            &id,
            &role,
            &content,
            &source_type,
        )
        .await
    }

    async fn get_session_history(
        &self,
        session_id: Uuid,
        limit: u32,
    ) -> Result<Vec<ChatMessageRow>> {
        crate::db::get_session_history(&self.pool, session_id, limit).await
    }

    async fn update_session_title(&self, session_id: Uuid, title: &str) -> Result<()> {
        crate::db::update_session_title(&self.pool, session_id, title).await
    }

    async fn update_session_summary(&self, session_id: Uuid, summary: &str) -> Result<()> {
        sqlx::query("UPDATE chat_sessions SET summary = $1 WHERE id = $2")
            .bind(summary)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count_session_messages(&self, session_id: Uuid) -> Result<u32> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM chat_messages WHERE session_id = $1")
                .bind(session_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0 as u32)
    }

    async fn update_session_draft_config(
        &self,
        session_id: Uuid,
        draft_config: Option<serde_json::Value>,
    ) -> Result<()> {
        crate::db::update_session_draft_config(&self.pool, session_id, draft_config).await
    }

    async fn clear_session_messages(&self, session_id: Uuid) -> Result<()> {
        crate::db::clear_session_messages(&self.pool, session_id).await
    }

    async fn find_session_by_step_id(&self, step_id: Uuid) -> Result<Option<SessionRow>> {
        crate::db::find_session_by_step_id(&self.pool, step_id).await
    }

    async fn find_builder_session_by_step_id(&self, step_id: Uuid) -> Result<Option<SessionRow>> {
        crate::db::find_builder_session_by_step_id(&self.pool, step_id).await
    }

    async fn find_manager_builder_session(&self, workflow_id: Uuid) -> Result<Option<SessionRow>> {
        crate::db::find_manager_builder_session(&self.pool, workflow_id).await
    }

    async fn check_initial_instructions_sent(
        &self,
        step_ids: &[Uuid],
    ) -> Result<std::collections::HashSet<Uuid>> {
        crate::db::check_initial_instructions_sent(&self.pool, step_ids).await
    }

    async fn link_session_agent(&self, session_id: Uuid, agent_id: Uuid) -> Result<()> {
        crate::db::link_session_agent(&self.pool, session_id, agent_id).await
    }
}
