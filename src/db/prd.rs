//! PRD database operations

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::types::{
    DataModelSketch, MilestoneSpec, PRDDocument, PRDId, PRDStatus, TechnicalDecision,
};

/// Save a PRD to the database (insert or update on conflict)
pub async fn save_prd(pool: &PgPool, prd: &PRDDocument) -> Result<()> {
    let status = prd.status.to_string();
    let success_criteria = serde_json::to_value(&prd.success_criteria)?;
    let technical_decisions = serde_json::to_value(&prd.technical_decisions)?;
    let data_models = serde_json::to_value(&prd.data_models)?;
    let milestones = serde_json::to_value(&prd.milestones)?;

    sqlx::query(
        r#"
        INSERT INTO prds (id, title, status, vision, problem_statement, target_users,
                          success_criteria, technical_decisions, data_models, milestones,
                          created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            status = excluded.status,
            vision = excluded.vision,
            problem_statement = excluded.problem_statement,
            target_users = excluded.target_users,
            success_criteria = excluded.success_criteria,
            technical_decisions = excluded.technical_decisions,
            data_models = excluded.data_models,
            milestones = excluded.milestones,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(prd.id.0)
    .bind(&prd.title)
    .bind(&status)
    .bind(&prd.vision)
    .bind(&prd.problem_statement)
    .bind(&prd.target_users)
    .bind(&success_criteria)
    .bind(&technical_decisions)
    .bind(&data_models)
    .bind(&milestones)
    .bind(prd.created_at)
    .bind(prd.updated_at)
    .execute(pool)
    .await
    .context("Failed to save PRD")?;

    Ok(())
}

/// Load a PRD by ID
pub async fn load_prd(pool: &PgPool, id: &PRDId) -> Result<Option<PRDDocument>> {
    let row: Option<PrdRow> = sqlx::query_as(
        "SELECT id, title, status, vision, problem_statement, target_users, \
         success_criteria, technical_decisions, data_models, milestones, \
         created_at, updated_at FROM prds WHERE id = $1",
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await
    .context("Failed to load PRD")?;

    match row {
        Some(row) => Ok(Some(row.into_prd())),
        None => Ok(None),
    }
}

/// List PRDs by status
pub async fn list_prds_by_status(pool: &PgPool, status: PRDStatus) -> Result<Vec<PRDDocument>> {
    let status_str = status.to_string();

    let rows: Vec<PrdRow> = sqlx::query_as(
        "SELECT id, title, status, vision, problem_statement, target_users, \
         success_criteria, technical_decisions, data_models, milestones, \
         created_at, updated_at FROM prds WHERE status = $1 ORDER BY updated_at DESC",
    )
    .bind(&status_str)
    .fetch_all(pool)
    .await
    .context("Failed to list PRDs")?;

    Ok(rows.into_iter().map(|r| r.into_prd()).collect())
}

#[derive(sqlx::FromRow)]
struct PrdRow {
    id: uuid::Uuid,
    title: String,
    status: String,
    vision: String,
    problem_statement: String,
    target_users: String,
    success_criteria: serde_json::Value,
    technical_decisions: serde_json::Value,
    data_models: serde_json::Value,
    milestones: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PrdRow {
    fn into_prd(self) -> PRDDocument {
        let status: PRDStatus = self.status.parse().unwrap_or(PRDStatus::Draft);

        let success_criteria: Vec<String> =
            serde_json::from_value(self.success_criteria).unwrap_or_default();
        let technical_decisions: Vec<TechnicalDecision> =
            serde_json::from_value(self.technical_decisions).unwrap_or_default();
        let data_models: Vec<DataModelSketch> =
            serde_json::from_value(self.data_models).unwrap_or_default();
        let milestones: Vec<MilestoneSpec> =
            serde_json::from_value(self.milestones).unwrap_or_default();

        PRDDocument {
            id: PRDId(self.id),
            title: self.title,
            status,
            vision: self.vision,
            problem_statement: self.problem_statement,
            target_users: self.target_users,
            success_criteria,
            technical_decisions,
            data_models,
            milestones,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MilestoneSpec;

    async fn setup_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://nexor:nexor@localhost:5432/nexor_test".to_string());
        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        sqlx::query("DELETE FROM prds")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn save_and_load_prd() {
        let pool = setup_db().await;

        let mut prd = PRDDocument::new("Test PRD");
        prd.vision = "Build something great".into();
        prd.milestones.push(MilestoneSpec {
            title: "M1".into(),
            description: "First".into(),
            deliverables: vec!["API".into()],
            dependencies: vec![],
        });

        save_prd(&pool, &prd).await.unwrap();
        let loaded = load_prd(&pool, &prd.id).await.unwrap().unwrap();

        assert_eq!(loaded.title, "Test PRD");
        assert_eq!(loaded.vision, "Build something great");
        assert_eq!(loaded.milestones.len(), 1);
        assert_eq!(loaded.milestones[0].title, "M1");

        pool.close().await;
    }

    #[tokio::test]
    async fn save_updates_existing() {
        let pool = setup_db().await;

        let mut prd = PRDDocument::new("Test PRD");
        save_prd(&pool, &prd).await.unwrap();

        prd.vision = "Updated vision".into();
        save_prd(&pool, &prd).await.unwrap();

        let loaded = load_prd(&pool, &prd.id).await.unwrap().unwrap();
        assert_eq!(loaded.vision, "Updated vision");

        pool.close().await;
    }

    #[tokio::test]
    async fn list_by_status() {
        let pool = setup_db().await;

        let prd1 = PRDDocument::new("Draft PRD");
        save_prd(&pool, &prd1).await.unwrap();

        let mut prd2 = PRDDocument::new("Approved PRD");
        prd2.status = PRDStatus::Approved;
        save_prd(&pool, &prd2).await.unwrap();

        let drafts = list_prds_by_status(&pool, PRDStatus::Draft).await.unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].title, "Draft PRD");

        let approved = list_prds_by_status(&pool, PRDStatus::Approved)
            .await
            .unwrap();
        assert_eq!(approved.len(), 1);
        assert_eq!(approved[0].title, "Approved PRD");

        pool.close().await;
    }

    #[tokio::test]
    async fn load_nonexistent_returns_none() {
        let pool = setup_db().await;
        let result = load_prd(&pool, &PRDId::new()).await.unwrap();
        assert!(result.is_none());
        pool.close().await;
    }
}
