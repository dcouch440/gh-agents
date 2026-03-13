use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::{AuthConfigRepo, ChatMessageRepo};
use crate::db::ChatMessageRow;
use crate::types::UserId;

use super::PgRepo;

// ============================================================================
// Auth Config Repository
// ============================================================================
#[async_trait]
impl AuthConfigRepo for PgRepo {
    async fn health_check(&self) -> bool {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await.is_ok()
    }

    async fn has_password(&self) -> Result<bool> {
        crate::db::has_password(&self.pool).await
    }

    async fn set_password(&self, password_hash: String) -> Result<()> {
        crate::db::set_password(&self.pool, &password_hash).await
    }

    async fn get_password(&self) -> Result<Option<String>> {
        crate::db::get_password(&self.pool).await
    }
}

// ============================================================================
// Chat Message Repository
// ============================================================================

#[async_trait]
impl ChatMessageRepo for PgRepo {
    async fn insert_chat_message(
        &self,
        user_id: UserId,
        id: Uuid,
        role: String,
        content: String,
    ) -> Result<()> {
        crate::db::insert_chat_message(&self.pool, user_id, &id, &role, &content).await
    }

    async fn get_chat_history(
        &self,
        user_id: UserId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ChatMessageRow>> {
        crate::db::get_chat_history(&self.pool, user_id, limit, offset).await
    }

    async fn clear_chat_history(&self, user_id: UserId) -> Result<()> {
        crate::db::clear_chat_history(&self.pool, user_id).await
    }
}
