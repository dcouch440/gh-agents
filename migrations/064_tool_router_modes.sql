-- ============================================================================
-- Migration 064: Runtime Router Modes System
-- ============================================================================
--
-- Purpose: Enable runtime personality/behavior selection based on user input.
--
-- What this adds:
--   1. Hierarchical routing support (L1 -> L2 -> L3) via parent_router_id
--   2. Router modes (configurations for system_prompt, temp, tokens, tools)
--   3. Mode-tool junction table (which tools each mode has access to)
--   4. Agent-router linking (agents.router_id)
--   5. Execution tracking (agent_executions.selected_router_mode_id)
--   6. Deprecation marking for old agent_modes system
--
-- Design:
--   - Extends existing tool_routers table (adds parent_router_id, level)
--   - New tool_router_modes table (mode configurations)
--   - New tool_router_mode_tools junction table (mode -> tools)
--   - Agents can optionally link to a router via router_id
--   - Execution records track which mode was selected
--
-- Related: See ROUTER_MODES_DESIGN.md for full architecture
-- ============================================================================

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 1: Extend tool_routers with Hierarchy Support
-- ────────────────────────────────────────────────────────────────────────────

-- Add parent_router_id for hierarchical routing (L1 -> L2 -> L3)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'tool_routers' AND column_name = 'parent_router_id'
    ) THEN
        ALTER TABLE tool_routers
        ADD COLUMN parent_router_id UUID REFERENCES tool_routers(id) ON DELETE CASCADE;
    END IF;
END $$;

-- Add level column (1, 2, or 3)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'tool_routers' AND column_name = 'level'
    ) THEN
        ALTER TABLE tool_routers
        ADD COLUMN level INT NOT NULL DEFAULT 1 CHECK (level IN (1, 2, 3));
    END IF;
END $$;

-- Index for parent lookups
CREATE INDEX IF NOT EXISTS idx_tool_routers_parent ON tool_routers(parent_router_id);

-- Index for level filtering
CREATE INDEX IF NOT EXISTS idx_tool_routers_level ON tool_routers(level);

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 2: Create tool_router_modes Table
-- ────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS tool_router_modes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    router_id UUID NOT NULL REFERENCES tool_routers(id) ON DELETE CASCADE,

    -- Mode identity
    mode_key TEXT NOT NULL,                  -- e.g., "coding", "research", "chat"
    display_name TEXT NOT NULL,              -- e.g., "Coding Mode"
    description TEXT NOT NULL,               -- For router LLM classification

    -- Behavior configuration
    system_prompt TEXT NOT NULL,             -- System prompt for this mode
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens INT NOT NULL DEFAULT 4096,

    -- Append vs Replace behavior
    append_to_agent_system_prompt BOOLEAN NOT NULL DEFAULT FALSE,
    append_to_agent_tools BOOLEAN NOT NULL DEFAULT TRUE,

    -- Metadata
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT unique_mode_key_per_router UNIQUE (router_id, mode_key),
    CHECK (mode_key ~ '^[a-z][a-z0-9_]*$'),
    CHECK (temperature BETWEEN 0.0 AND 2.0),
    CHECK (max_tokens > 0)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_tool_router_modes_router ON tool_router_modes(router_id);
CREATE INDEX IF NOT EXISTS idx_tool_router_modes_order ON tool_router_modes(router_id, display_order);

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 3: Create tool_router_mode_tools Junction Table
-- ────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS tool_router_mode_tools (
    mode_id UUID NOT NULL REFERENCES tool_router_modes(id) ON DELETE CASCADE,
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (mode_id, tool_id)
);

-- Index for reverse lookups (which modes use this tool)
CREATE INDEX IF NOT EXISTS idx_tool_router_mode_tools_tool ON tool_router_mode_tools(tool_id);

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 4: Add router_id to agents Table
-- ────────────────────────────────────────────────────────────────────────────

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'agents' AND column_name = 'router_id'
    ) THEN
        ALTER TABLE agents
        ADD COLUMN router_id UUID REFERENCES tool_routers(id) ON DELETE SET NULL;
    END IF;
END $$;

-- Index for router lookups
CREATE INDEX IF NOT EXISTS idx_agents_router ON agents(router_id);

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 5: Add selected_router_mode_id to agent_executions Table
-- ────────────────────────────────────────────────────────────────────────────

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'agent_executions' AND column_name = 'selected_router_mode_id'
    ) THEN
        ALTER TABLE agent_executions
        ADD COLUMN selected_router_mode_id UUID REFERENCES tool_router_modes(id) ON DELETE SET NULL;
    END IF;
END $$;

-- Index for analytics queries
CREATE INDEX IF NOT EXISTS idx_agent_executions_router_mode ON agent_executions(selected_router_mode_id);

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 6: Deprecate agent_modes Table
-- ────────────────────────────────────────────────────────────────────────────

-- Mark table as deprecated (don't drop yet - migration happens in Phase 10)
COMMENT ON TABLE agent_modes IS
'DEPRECATED: Use tool_router_modes instead.
Migrate data via Phase 10 migration script.
Will be dropped after verification.
DO NOT add new agent_modes - use tool_router_modes.';

-- ────────────────────────────────────────────────────────────────────────────
-- VERIFICATION
-- ────────────────────────────────────────────────────────────────────────────

DO $$
DECLARE
    tool_routers_parent_exists BOOLEAN;
    tool_routers_level_exists BOOLEAN;
    tool_router_modes_exists BOOLEAN;
    tool_router_mode_tools_exists BOOLEAN;
    agents_router_id_exists BOOLEAN;
    executions_router_mode_id_exists BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'tool_routers' AND column_name = 'parent_router_id'
    ) INTO tool_routers_parent_exists;

    SELECT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'tool_routers' AND column_name = 'level'
    ) INTO tool_routers_level_exists;

    SELECT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'tool_router_modes'
    ) INTO tool_router_modes_exists;

    SELECT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'tool_router_mode_tools'
    ) INTO tool_router_mode_tools_exists;

    SELECT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'agents' AND column_name = 'router_id'
    ) INTO agents_router_id_exists;

    SELECT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'agent_executions' AND column_name = 'selected_router_mode_id'
    ) INTO executions_router_mode_id_exists;

    IF tool_routers_parent_exists AND
       tool_routers_level_exists AND
       tool_router_modes_exists AND
       tool_router_mode_tools_exists AND
       agents_router_id_exists AND
       executions_router_mode_id_exists THEN
        RAISE NOTICE 'Migration 064 verified: all objects present';
    ELSE
        RAISE WARNING 'Migration 064 incomplete: some objects missing';
    END IF;
END $$;
