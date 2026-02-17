#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::MockTokenLedgerRepo;
    use crate::server::services::costs::*;

    #[tokio::test]
    async fn get_costs_returns_breakdown() {
        let mut repo = MockTokenLedgerRepo::new();
        repo.expect_get_user_spend().returning(|_, _| Ok(12.50));
        repo.expect_get_model_breakdown()
            .returning(|_, _| Ok(vec![]));

        let result = get_costs(&repo, Uuid::new_v4(), None).await;
        let breakdown = result.unwrap();
        assert!((breakdown.total_spend - 12.50).abs() < f64::EPSILON);
        assert!(breakdown.models.is_empty());
    }
}
