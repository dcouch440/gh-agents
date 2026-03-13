use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::{SystemConfigRow, ToolCapabilityRow, ToolRow};

// ============================================================================
// System Configuration Repository (Phase 3)
// ============================================================================

/// Repository for system-wide configuration (admin-controlled)
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SystemConfigRepo: Send + Sync {
    // Basic config operations

    /// Get a system config by key
    async fn get_system_config(&self, config_key: &str) -> Result<Option<SystemConfigRow>>;

    /// List system configs, optionally filtered by type
    async fn list_system_configs(
        &self,
        config_type: Option<String>,
    ) -> Result<Vec<SystemConfigRow>>;

    /// Upsert a system config (insert or update)
    async fn upsert_system_config(
        &self,
        config_type: &str,
        config_key: &str,
        config_value: &serde_json::Value,
        description: Option<String>,
        created_by: Option<Uuid>,
    ) -> Result<SystemConfigRow>;

    /// Delete a system config
    async fn delete_system_config(&self, config_key: &str) -> Result<()>;

    // Specialized config queries

    /// Get all execution constraints as a map
    async fn get_execution_constraints(
        &self,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>>;

    /// Check if unsafe operations are enabled
    async fn get_unsafe_operations_enabled(&self) -> Result<bool>;
}

// ============================================================================
// Tool Capability Repository (Phase 3)
// ============================================================================

/// Repository for tool capability taxonomy and assignments
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ToolCapabilityRepo: Send + Sync {
    // Capability taxonomy queries

    /// Get all tool capabilities
    async fn get_tool_capabilities(&self) -> Result<Vec<ToolCapabilityRow>>;

    /// Get a capability by ID
    async fn get_tool_capability(&self, id: Uuid) -> Result<Option<ToolCapabilityRow>>;

    /// Get a capability by key
    async fn get_tool_capability_by_key(&self, key: &str) -> Result<Option<ToolCapabilityRow>>;

    // Tool-to-capability assignments

    /// Get all capabilities assigned to a tool
    async fn get_capabilities_by_tool(&self, tool_id: Uuid) -> Result<Vec<ToolCapabilityRow>>;

    /// Get all tools that provide a capability
    async fn get_tools_by_capability(&self, capability_key: &str) -> Result<Vec<ToolRow>>;

    /// Get all tools that provide ANY of the given capabilities (union, deduplicated)
    async fn get_tools_by_capabilities(&self, capability_keys: &[String]) -> Result<Vec<ToolRow>>;

    /// Assign a capability to a tool
    async fn assign_capability_to_tool(&self, tool_id: Uuid, capability_id: Uuid) -> Result<()>;

    /// Remove a capability from a tool
    async fn remove_capability_from_tool(&self, tool_id: Uuid, capability_id: Uuid) -> Result<()>;

    /// Set all capabilities for a tool (replaces existing)
    async fn set_tool_capabilities(&self, tool_id: Uuid, capability_ids: &[Uuid]) -> Result<()>;

    // Mode-to-capability requirements

    /// Get all capabilities required by a mode
    async fn get_mode_capabilities(&self, mode_id: Uuid) -> Result<Vec<ToolCapabilityRow>>;

    /// Set capabilities required by a mode (replaces existing)
    async fn set_mode_capabilities(
        &self,
        mode_id: Uuid,
        capability_ids: &[Uuid],
        is_required: bool,
    ) -> Result<()>;
}
