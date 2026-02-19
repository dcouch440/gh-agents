#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::super::{check_direct_owner, check_strict_owner, check_system_passthrough};
    use crate::server::services::ServiceError;

    fn user_a() -> Uuid {
        Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap()
    }

    fn user_b() -> Uuid {
        Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap()
    }

    // ── check_direct_owner ──────────────────────────────────────────────

    #[test]
    fn direct_owner_passes_when_equal() {
        assert!(check_direct_owner(user_a(), user_a(), "Widget").is_ok());
    }

    #[test]
    fn direct_owner_fails_when_different() {
        let err = check_direct_owner(user_a(), user_b(), "Widget").unwrap_err();
        assert!(matches!(err, ServiceError::NotFound(ref s) if s == "Widget"));
    }

    // ── check_system_passthrough ────────────────────────────────────────

    #[test]
    fn system_passthrough_passes_for_none() {
        assert!(check_system_passthrough(None, user_a(), "Agent").is_ok());
    }

    #[test]
    fn system_passthrough_passes_for_matching_owner() {
        assert!(check_system_passthrough(Some(user_a()), user_a(), "Agent").is_ok());
    }

    #[test]
    fn system_passthrough_fails_for_mismatched_owner() {
        let err = check_system_passthrough(Some(user_a()), user_b(), "Agent").unwrap_err();
        assert!(matches!(err, ServiceError::NotFound(ref s) if s == "Agent"));
    }

    // ── check_strict_owner ──────────────────────────────────────────────

    #[test]
    fn strict_owner_passes_for_matching_owner() {
        assert!(check_strict_owner(Some(user_a()), user_a(), "Template").is_ok());
    }

    #[test]
    fn strict_owner_fails_for_none() {
        let err = check_strict_owner(None, user_a(), "Template").unwrap_err();
        assert!(matches!(err, ServiceError::NotFound(ref s) if s == "Template"));
    }

    #[test]
    fn strict_owner_fails_for_mismatched_owner() {
        let err = check_strict_owner(Some(user_a()), user_b(), "Template").unwrap_err();
        assert!(matches!(err, ServiceError::NotFound(ref s) if s == "Template"));
    }
}
