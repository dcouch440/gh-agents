//! Centralized ownership verification helpers.
//!
//! Three pure check functions cover all ownership semantics in the service layer:
//!
//! - [`check_direct_owner`] — row has a non-optional `Uuid` owner
//! - [`check_system_passthrough`] — `None` means system resource (visible to all)
//! - [`check_strict_owner`] — `None` means system resource (read-only, not mutable)

use std::future::Future;

use uuid::Uuid;

use super::ServiceError;

/// Verify that `caller` owns a resource with a non-optional `user_id`.
///
/// Used by: workflows, sessions, rooms, documents, collections.
pub fn check_direct_owner(owner: Uuid, caller: Uuid, entity: &str) -> Result<(), ServiceError> {
    if owner != caller {
        return Err(ServiceError::not_found(entity));
    }
    Ok(())
}

/// Verify ownership when `None` means "system resource, visible to all users".
///
/// - `None` → passes (system resource, accessible to everyone)
/// - `Some(owner) == caller` → passes
/// - `Some(owner) != caller` → fails with not-found
///
/// Used by: agents, tools (agent ownership gate).
pub fn check_system_passthrough(
    owner: Option<Uuid>,
    caller: Uuid,
    entity: &str,
) -> Result<(), ServiceError> {
    if owner.is_some() && owner != Some(caller) {
        return Err(ServiceError::not_found(entity));
    }
    Ok(())
}

/// Verify ownership when `None` means "system resource, not user-mutable".
///
/// - `Some(owner) == caller` → passes
/// - `Some(owner) != caller` → fails with not-found
/// - `None` → fails with not-found (system resources cannot be mutated)
///
/// Used by: prompt_templates, output_schemas (update/delete gates).
pub fn check_strict_owner(
    owner: Option<Uuid>,
    caller: Uuid,
    entity: &str,
) -> Result<(), ServiceError> {
    match owner {
        Some(o) if o == caller => Ok(()),
        _ => Err(ServiceError::not_found(entity)),
    }
}

/// Fetch an entity and verify direct ownership in one step.
///
/// Combines the common pattern: fetch row → not_found if missing → check owner.
/// Used by services that guard resources via `check_direct_owner`.
pub(crate) async fn fetch_and_check_owner<T, F, Fut>(
    fetch: F,
    caller: Uuid,
    get_owner: impl FnOnce(&T) -> Uuid,
    entity_name: &str,
) -> Result<T, ServiceError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Option<T>, anyhow::Error>>,
{
    let entity = fetch()
        .await?
        .ok_or_else(|| ServiceError::not_found(entity_name))?;
    check_direct_owner(get_owner(&entity), caller, entity_name)?;
    Ok(entity)
}

#[cfg(test)]
mod tests;
