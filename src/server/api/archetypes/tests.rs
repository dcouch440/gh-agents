#[cfg(test)]
mod tests {
    use super::super::{list_archetypes, ARCHETYPES};

    #[tokio::test]
    async fn list_archetypes_returns_all() {
        let result = list_archetypes().await;
        assert_eq!(result.0.len(), 3);
    }

    #[test]
    fn all_archetypes_have_required_fields() {
        for a in ARCHETYPES {
            assert!(!a.id.is_empty());
            assert!(!a.name.is_empty());
            assert!(!a.description.is_empty());
            assert!(!a.icon.is_empty());
            assert!(a.color.starts_with('#'));
        }
    }

    #[test]
    fn archetype_ids_are_unique() {
        let ids: Vec<&str> = ARCHETYPES.iter().map(|a| a.id).collect();
        let mut deduped = ids.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len());
    }
}
