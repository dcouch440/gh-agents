//! PRD database operations

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::types::{
    DataModelSketch, MilestoneSpec, PRDDocument, PRDId, PRDStatus, TechnicalDecision,
};

/// Save a PRD to the database (insert or update on conflict)
pub async fn save_prd(pool: &SqlitePool, prd: &PRDDocument) -> Result<()> {
    let id = prd.id.0.to_string();
    let status = prd.status.to_string();
    let success_criteria = serde_json::to_string(&prd.success_criteria)?;
    let technical_decisions = serde_json::to_string(&prd.technical_decisions)?;
    let data_models = serde_json::to_string(&prd.data_models)?;
    let milestones = serde_json::to_string(&prd.milestones)?;
    let created_at = prd.created_at.to_rfc3339();
    let updated_at = prd.updated_at.to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO prds (id, title, status, vision, problem_statement, target_users,
                          success_criteria, technical_decisions, data_models, milestones,
                          created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .bind(&id)
    .bind(&prd.title)
    .bind(&status)
    .bind(&prd.vision)
    .bind(&prd.problem_statement)
    .bind(&prd.target_users)
    .bind(&success_criteria)
    .bind(&technical_decisions)
    .bind(&data_models)
    .bind(&milestones)
    .bind(&created_at)
    .bind(&updated_at)
    .execute(pool)
    .await
    .context("Failed to save PRD")?;

    Ok(())
}

/// Load a PRD by ID
pub async fn load_prd(pool: &SqlitePool, id: &PRDId) -> Result<Option<PRDDocument>> {
    let id_str = id.0.to_string();

    let row: Option<PrdRow> = sqlx::query_as(
        "SELECT id, title, status, vision, problem_statement, target_users, \
         success_criteria, technical_decisions, data_models, milestones, \
         created_at, updated_at FROM prds WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_optional(pool)
    .await
    .context("Failed to load PRD")?;

    match row {
        Some(row) => Ok(Some(row.into_prd()?)),
        None => Ok(None),
    }
}

/// List PRDs by status
pub async fn list_prds_by_status(pool: &SqlitePool, status: PRDStatus) -> Result<Vec<PRDDocument>> {
    let status_str = status.to_string();

    let rows: Vec<PrdRow> = sqlx::query_as(
        "SELECT id, title, status, vision, problem_statement, target_users, \
         success_criteria, technical_decisions, data_models, milestones, \
         created_at, updated_at FROM prds WHERE status = ? ORDER BY updated_at DESC",
    )
    .bind(&status_str)
    .fetch_all(pool)
    .await
    .context("Failed to list PRDs")?;

    rows.into_iter().map(|r| r.into_prd()).collect()
}

#[derive(sqlx::FromRow)]
struct PrdRow {
    id: String,
    title: String,
    status: String,
    vision: String,
    problem_statement: String,
    target_users: String,
    success_criteria: String,
    technical_decisions: String,
    data_models: String,
    milestones: String,
    created_at: String,
    updated_at: String,
}

impl PrdRow {
    fn into_prd(self) -> Result<PRDDocument> {
        let id = uuid::Uuid::parse_str(&self.id).context("Invalid PRD ID")?;
        let status: PRDStatus = self
            .status
            .parse()
            .map_err(|e: String| anyhow::anyhow!(e))?;
        let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&self.created_at)
            .context("Invalid created_at timestamp")?
            .with_timezone(&Utc);
        let updated_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&self.updated_at)
            .context("Invalid updated_at timestamp")?
            .with_timezone(&Utc);

        let success_criteria: Vec<String> =
            serde_json::from_str(&self.success_criteria).unwrap_or_default();
        let technical_decisions: Vec<TechnicalDecision> =
            serde_json::from_str(&self.technical_decisions).unwrap_or_default();
        let data_models: Vec<DataModelSketch> =
            serde_json::from_str(&self.data_models).unwrap_or_default();
        let milestones: Vec<MilestoneSpec> =
            serde_json::from_str(&self.milestones).unwrap_or_default();

        Ok(PRDDocument {
            id: PRDId(id),
            title: self.title,
            status,
            vision: self.vision,
            problem_statement: self.problem_statement,
            target_users: self.target_users,
            success_criteria,
            technical_decisions,
            data_models,
            milestones,
            created_at,
            updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MilestoneSpec;
    use tempfile::TempDir;

    async fn setup_db() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = crate::db::init_db_at(db_path.to_str().unwrap())
            .await
            .unwrap();
        (pool, temp_dir)
    }

    #[tokio::test]
    async fn save_and_load_prd() {
        let (pool, _dir) = setup_db().await;

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
        let (pool, _dir) = setup_db().await;

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
        let (pool, _dir) = setup_db().await;

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
        let (pool, _dir) = setup_db().await;
        let result = load_prd(&pool, &PRDId::new()).await.unwrap();
        assert!(result.is_none());
        pool.close().await;
    }
}
