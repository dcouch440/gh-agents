#[cfg(test)]
mod tests {
    // Note: save_version, list_versions, and restore_version are async functions
    // that depend on AppState + real DB (capture_workflow_snapshot, restore, etc.).
    // Unit tests for the pure logic are in the calling layers.
    //
    // Integration tests would require a running database. For now, the version
    // service is tested via the API handlers in the full test suite.
    //
    // The DB layer (create/list/get/delete workflow_version) is tested by the
    // pg_repo tests when a migration is applied.
}
