#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use uuid::Uuid;

    use crate::server::hub::dag::container::extract_step_overlay;

    /// `extract_step_overlay` is now called on the failure path too, between a
    /// failed agent level and container teardown. It must stay total: a missing
    /// container or a disabled overlay returns None rather than panicking
    /// mid-teardown and losing the very files the reordering exists to save.
    #[tokio::test]
    async fn extract_step_overlay_is_none_without_a_container() {
        let out = extract_step_overlay(
            &None,
            Uuid::new_v4(),
            "step".to_string(),
            "description".to_string(),
            0,
            &HashSet::new(),
            true,
        )
        .await;
        assert!(out.is_none());
    }

    #[tokio::test]
    async fn extract_step_overlay_is_none_when_overlay_disabled() {
        let out = extract_step_overlay(
            &None,
            Uuid::new_v4(),
            "step".to_string(),
            "description".to_string(),
            0,
            &HashSet::new(),
            false,
        )
        .await;
        assert!(out.is_none());
    }
}
