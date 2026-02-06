-- ============================================================================
-- Migration 071: Master System Config - Admin-Controlled Truth
-- ============================================================================
-- Purpose: Create admin-controlled master configuration system.
--          Define execution constraints, capability taxonomy references,
--          and routing strategies.
--
-- New Tables:
--   - system_config: Key-value store for system-wide configuration
--
-- Seed Data:
--   - Default execution constraints (max_subtasks, timeouts, cost limits)
--   - Safety configuration (unsafe operations toggle)
--   - Capability metadata references
-- ============================================================================

-- ============================================================================
-- 1. CREATE TABLES
-- ============================================================================

-- System configuration (admin-only)
CREATE TABLE system_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    config_type TEXT NOT NULL,  -- "capability", "constraint", "routing_strategy", "system_agent"
    config_key TEXT NOT NULL UNIQUE,  -- Hierarchical key (e.g., "constraint:max_subtasks", "unsafe_operations_enabled")
    config_value JSONB NOT NULL,
    description TEXT,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_system_config_type ON system_config(config_type);
CREATE INDEX idx_system_config_key ON system_config(config_key);

COMMENT ON TABLE system_config IS
    'Master system configuration (admin-controlled). Defines capabilities, execution constraints, routing strategies, and system agents.';

-- ============================================================================
-- 2. SEED EXECUTION CONSTRAINTS
-- ============================================================================

INSERT INTO system_config (config_type, config_key, config_value, description) VALUES
    (
        'constraint',
        'max_subtasks_per_cavernous_step',
        '10'::jsonb,
        'Maximum number of subtasks allowed in a single cavernous routing execution'
    ),
    (
        'constraint',
        'max_cavernous_nesting_depth',
        '3'::jsonb,
        'Maximum depth of nested cavernous routing (prevents infinite recursion)'
    ),
    (
        'constraint',
        'max_execution_time_minutes',
        '60'::jsonb,
        'Maximum execution time for a single workflow execution in minutes'
    ),
    (
        'constraint',
        'max_cost_per_execution_usd',
        '10.0'::jsonb,
        'Maximum cost (in USD) for a single workflow execution'
    ),
    (
        'constraint',
        'max_tokens_per_step',
        '100000'::jsonb,
        'Maximum tokens (input + output) allowed for a single step execution'
    ),
    (
        'constraint',
        'unsafe_operations_enabled',
        'false'::jsonb,
        'Enable unsafe operations (shell execution, dangerous commands). Admin can enable per-tenant.'
    ),
    (
        'constraint',
        'dangerous_tools_require_approval',
        'true'::jsonb,
        'Require explicit approval before executing dangerous tools (safety_level=unsafe)'
    ),
    (
        'constraint',
        'max_parallel_for_each',
        '20'::jsonb,
        'Maximum number of parallel iterations in for-each steps'
    )
ON CONFLICT (config_key) DO NOTHING;

-- ============================================================================
-- 3. SEED CAPABILITY METADATA REFERENCES
-- ============================================================================
-- Links system_config to tool_capabilities for unified configuration view

INSERT INTO system_config (config_type, config_key, config_value, description)
SELECT
    'capability_meta',
    'capability:' || capability_key,
    jsonb_build_object(
        'id', id,
        'display_name', display_name,
        'category', category,
        'safety_level', safety_level
    ),
    'Reference to capability: ' || description
FROM tool_capabilities
ON CONFLICT (config_key) DO NOTHING;
