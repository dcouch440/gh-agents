//! PRD database operations

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::types::{DataModelSketch, MilestoneSpec, PRDDocument, PRDId, PRDStatus, TechnicalDecision};

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

        let success_criteria: Vec<String> = serde_json::from_value(self.success_criteria).unwrap_or_default();
        let technical_decisions: Vec<TechnicalDecision> = serde_json::from_value(self.technical_decisions).unwrap_or_default();
        let data_models: Vec<DataModelSketch> = serde_json::from_value(self.data_models).unwrap_or_default();
        let milestones: Vec<MilestoneSpec> = serde_json::from_value(self.milestones).unwrap_or_default();

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

    use crate::db::test_utils::TestDb;

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn save_and_load_prd() {
        let db = TestDb::new().await;

        let mut prd = PRDDocument::new("Test PRD");
        prd.vision = "Build something great".into();
        prd.milestones.push(MilestoneSpec {
            title: "M1".into(),
            description: "First".into(),
            deliverables: vec!["API".into()],
            dependencies: vec![],
        });

        save_prd(&db.pool, &prd).await.unwrap();
        let loaded = load_prd(&db.pool, &prd.id).await.unwrap().unwrap();

        assert_eq!(loaded.title, "Test PRD");
        assert_eq!(loaded.vision, "Build something great");
        assert_eq!(loaded.milestones.len(), 1);
        assert_eq!(loaded.milestones[0].title, "M1");

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn save_updates_existing() {
        let db = TestDb::new().await;

        let mut prd = PRDDocument::new("Test PRD");
        save_prd(&db.pool, &prd).await.unwrap();

        prd.vision = "Updated vision".into();
        save_prd(&db.pool, &prd).await.unwrap();

        let loaded = load_prd(&db.pool, &prd.id).await.unwrap().unwrap();
        assert_eq!(loaded.vision, "Updated vision");

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn list_by_status() {
        let db = TestDb::new().await;

        let prd1 = PRDDocument::new("Draft PRD");
        save_prd(&db.pool, &prd1).await.unwrap();

        let mut prd2 = PRDDocument::new("Approved PRD");
        prd2.status = PRDStatus::Approved;
        save_prd(&db.pool, &prd2).await.unwrap();

        let drafts = list_prds_by_status(&db.pool, PRDStatus::Draft).await.unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].title, "Draft PRD");

        let approved = list_prds_by_status(&db.pool, PRDStatus::Approved).await.unwrap();
        assert_eq!(approved.len(), 1);
        assert_eq!(approved[0].title, "Approved PRD");

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn load_nonexistent_returns_none() {
        let db = TestDb::new().await;
        let result = load_prd(&db.pool, &PRDId::new()).await.unwrap();
        assert!(result.is_none());
        db.cleanup().await;
    }
}
