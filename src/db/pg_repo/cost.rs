use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::traits::{ModelSpendRow, ResultRepo, TokenLedgerRepo};
use crate::db::{ResultRow, TokenLedgerRow};

use super::PgRepo;

#[async_trait]
impl TokenLedgerRepo for PgRepo {
    async fn insert_ledger_entry(
        &self,
        user_id: Uuid,
        agent_execution_id: Option<Uuid>,
        model_id: &str,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f32,
    ) -> Result<TokenLedgerRow> {
        let row =
            sqlx::query_as::<_, TokenLedgerRow>("INSERT INTO token_ledger (user_id, agent_execution_id, model_id, input_tokens, output_tokens, cost_usd) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *")
                .bind(user_id)
                .bind(agent_execution_id)
                .bind(model_id)
                .bind(input_tokens)
                .bind(output_tokens)
                .bind(cost_usd)
                .fetch_one(&self.pool)
                .await?;
        Ok(row)
    }

    async fn get_user_spend(&self, user_id: Uuid, since: Option<DateTime<Utc>>) -> Result<f64> {
        let row: (Option<f64>,) = match since {
            Some(t) => {
                sqlx::query_as("SELECT CAST(COALESCE(SUM(cost_usd), 0) AS FLOAT8) FROM token_ledger WHERE user_id = $1 AND created_at >= $2")
                    .bind(user_id)
                    .bind(t)
                    .fetch_one(&self.pool)
                    .await?
            }
            None => {
                sqlx::query_as("SELECT CAST(COALESCE(SUM(cost_usd), 0) AS FLOAT8) FROM token_ledger WHERE user_id = $1")
                    .bind(user_id)
                    .fetch_one(&self.pool)
                    .await?
            }
        };
        Ok(row.0.unwrap_or(0.0))
    }

    async fn get_model_breakdown(
        &self,
        user_id: Uuid,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ModelSpendRow>> {
        let rows = match since {
            Some(t) => {
                sqlx::query_as::<_, ModelSpendRow>("SELECT model_id, CAST(SUM(input_tokens) AS INT8) AS total_input_tokens, CAST(SUM(output_tokens) AS INT8) AS total_output_tokens, CAST(SUM(cost_usd) AS FLOAT8) AS total_cost_usd, COUNT(*) AS call_count FROM token_ledger WHERE user_id = $1 AND created_at >= $2 GROUP BY model_id ORDER BY total_cost_usd DESC")
                    .bind(user_id)
                    .bind(t)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                sqlx::query_as::<_, ModelSpendRow>("SELECT model_id, CAST(SUM(input_tokens) AS INT8) AS total_input_tokens, CAST(SUM(output_tokens) AS INT8) AS total_output_tokens, CAST(SUM(cost_usd) AS FLOAT8) AS total_cost_usd, COUNT(*) AS call_count FROM token_ledger WHERE user_id = $1 GROUP BY model_id ORDER BY total_cost_usd DESC")
                    .bind(user_id)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        Ok(rows)
    }
}

#[async_trait]
impl ResultRepo for PgRepo {
    async fn save_result(
        &self,
        user_id: Uuid,
        agent_execution_id: Uuid,
        output_schema_id: Option<Uuid>,
        name: &str,
        data: serde_json::Value,
    ) -> Result<ResultRow> {
        let row = sqlx::query_as::<_, ResultRow>("INSERT INTO results (user_id, agent_execution_id, output_schema_id, name, data) VALUES ($1, $2, $3, $4, $5) RETURNING *")
            .bind(user_id)
            .bind(agent_execution_id)
            .bind(output_schema_id)
            .bind(name)
            .bind(data)
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }

    async fn get_result(&self, id: Uuid) -> Result<Option<ResultRow>> {
        let row = sqlx::query_as::<_, ResultRow>("SELECT * FROM results WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_results(&self, user_id: Uuid) -> Result<Vec<ResultRow>> {
        let rows = sqlx::query_as::<_, ResultRow>(
            "SELECT * FROM results WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_results_by_schema(
        &self,
        user_id: Uuid,
        output_schema_id: Uuid,
    ) -> Result<Vec<ResultRow>> {
        let rows = sqlx::query_as::<_, ResultRow>("SELECT * FROM results WHERE user_id = $1 AND output_schema_id = $2 ORDER BY created_at DESC")
            .bind(user_id)
            .bind(output_schema_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn delete_result(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM results WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
