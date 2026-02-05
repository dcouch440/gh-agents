#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Implement comprehensive test suite
    //
    // Tests should cover:
    // 1. CRUD operations (create, list, get, update, delete)
    // 2. Validation (invalid mode_key format, duplicates, ranges)
    // 3. Authorization (user isolation)
    // 4. Tool assignment (get_tools, set_tools)
    //
    // Test structure example:
    //
    // async fn setup_test_router(state: &AppState, user_id: Uuid) -> Uuid {
    //     // Create test router
    // }
    //
    // #[tokio::test]
    // async fn test_create_router_mode_success() {
    //     // POST with valid payload → 201 CREATED
    // }
    //
    // #[tokio::test]
    // async fn test_create_router_mode_invalid_key() {
    //     // mode_key with uppercase/spaces → 400 BAD_REQUEST
    // }
    //
    // #[tokio::test]
    // async fn test_create_router_mode_duplicate_key() {
    //     // Create mode twice with same key → 409 CONFLICT
    // }
    //
    // #[tokio::test]
    // async fn test_list_router_modes() {
    //     // Create 3 modes → GET returns all 3
    // }
    //
    // #[tokio::test]
    // async fn test_get_router_mode() {
    //     // Create mode → GET by ID → 200 with data
    // }
    //
    // #[tokio::test]
    // async fn test_update_router_mode() {
    //     // Create mode → PUT partial update → fields changed
    // }
    //
    // #[tokio::test]
    // async fn test_delete_router_mode() {
    //     // Create mode → DELETE → 204 → GET returns 404
    // }
    //
    // #[tokio::test]
    // async fn test_set_mode_tools() {
    //     // Create mode + tools → PUT /tools → GET /tools
    // }
    //
    // #[tokio::test]
    // async fn test_user_isolation() {
    //     // User A creates mode → User B tries GET → 404
    // }
}
