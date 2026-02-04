-- ============================================================================
-- Migration 064: Runtime Router Modes System
-- ============================================================================
--
-- Purpose: Enable runtime personality/behavior selection based on user input.
--
-- What this adds:
--   1. Hierarchical routing support (L1 → L2 → L3) via parent_router_id
--   2. Router modes (configurations for system_prompt, temp, tokens, tools)
--   3. Mode-tool junction table (which tools each mode has access to)
--   4. Agent-router linking (agents.router_id)
--   5. Execution tracking (agent_executions.selected_router_mode_id)
--   6. Deprecation marking for old agent_modes system
--
-- Design:
--   - Extends existing tool_routers table (adds parent_router_id, level)
--   - New tool_router_modes table (mode configurations)
--   - New tool_router_mode_tools junction table (mode → tools)
--   - Agents can optionally link to a router via router_id
--   - Execution records track which mode was selected
--
-- Related: See ROUTER_MODES_DESIGN.md for full architecture
-- ============================================================================

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 1: Extend tool_routers with Hierarchy Support
-- ────────────────────────────────────────────────────────────────────────────

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Step 1: Extending tool_routers';
    RAISE NOTICE '========================================';
END $$;

-- Add parent_router_id for hierarchical routing (L1 → L2 → L3)
ALTER TABLE tool_routers
ADD COLUMN parent_router_id UUID REFERENCES tool_routers(id) ON DELETE CASCADE;

-- Add level column (1, 2, or 3)
ALTER TABLE tool_routers
ADD COLUMN level INT NOT NULL DEFAULT 1 CHECK (level IN (1, 2, 3));

-- Index for parent lookups
CREATE INDEX idx_tool_routers_parent ON tool_routers(parent_router_id);

-- Index for level filtering
CREATE INDEX idx_tool_routers_level ON tool_routers(level);

DO $$
BEGIN
    RAISE NOTICE '✅ Added parent_router_id and level columns to tool_routers';
    RAISE NOTICE '✅ Created indexes: idx_tool_routers_parent, idx_tool_routers_level';
END $$;

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 2: Create tool_router_modes Table
-- ────────────────────────────────────────────────────────────────────────────

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Step 2: Creating tool_router_modes';
    RAISE NOTICE '========================================';
END $$;

CREATE TABLE tool_router_modes (
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
    append_to_agent_system_prompt BOOLEAN NOT NULL DEFAULT FALSE,  -- Append mode prompt to agent's or replace
    append_to_agent_tools BOOLEAN NOT NULL DEFAULT TRUE,           -- Add mode tools to agent's or replace

    -- Metadata
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT unique_mode_key_per_router UNIQUE (router_id, mode_key),
    CHECK (mode_key ~ '^[a-z][a-z0-9_]*$'),  -- snake_case only
    CHECK (temperature BETWEEN 0.0 AND 2.0),
    CHECK (max_tokens > 0)
);

-- Indexes
CREATE INDEX idx_tool_router_modes_router ON tool_router_modes(router_id);
CREATE INDEX idx_tool_router_modes_order ON tool_router_modes(router_id, display_order);

DO $$
BEGIN
    RAISE NOTICE '✅ Created tool_router_modes table';
    RAISE NOTICE '   - mode_key: snake_case identifier (unique per router)';
    RAISE NOTICE '   - display_name: Human-readable name';
    RAISE NOTICE '   - description: For router LLM classification';
    RAISE NOTICE '   - system_prompt: Personality for this mode';
    RAISE NOTICE '   - append_to_agent_system_prompt: Append or replace agent prompt';
    RAISE NOTICE '   - append_to_agent_tools: Union or replace agent tools';
    RAISE NOTICE '✅ Created indexes: idx_tool_router_modes_router, idx_tool_router_modes_order';
END $$;

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 3: Create tool_router_mode_tools Junction Table
-- ────────────────────────────────────────────────────────────────────────────

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Step 3: Creating tool_router_mode_tools';
    RAISE NOTICE '========================================';
END $$;

CREATE TABLE tool_router_mode_tools (
    mode_id UUID NOT NULL REFERENCES tool_router_modes(id) ON DELETE CASCADE,
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (mode_id, tool_id)
);

-- Index for reverse lookups (which modes use this tool)
CREATE INDEX idx_tool_router_mode_tools_tool ON tool_router_mode_tools(tool_id);

DO $$
BEGIN
    RAISE NOTICE '✅ Created tool_router_mode_tools junction table';
    RAISE NOTICE '   - Links modes to their available tools';
    RAISE NOTICE '   - CASCADE deletes when mode or tool is deleted';
    RAISE NOTICE '✅ Created index: idx_tool_router_mode_tools_tool';
END $$;

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 4: Add router_id to agents Table
-- ────────────────────────────────────────────────────────────────────────────

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Step 4: Linking agents to routers';
    RAISE NOTICE '========================================';
END $$;

ALTER TABLE agents
ADD COLUMN router_id UUID REFERENCES tool_routers(id) ON DELETE SET NULL;

-- Index for router lookups
CREATE INDEX idx_agents_router ON agents(router_id);

DO $$
BEGIN
    RAISE NOTICE '✅ Added router_id column to agents';
    RAISE NOTICE '   - NULL = agent uses default behavior (backward compatible)';
    RAISE NOTICE '   - SET NULL on router deletion (agent continues with defaults)';
    RAISE NOTICE '✅ Created index: idx_agents_router';
END $$;

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 5: Add selected_router_mode_id to agent_executions Table
-- ────────────────────────────────────────────────────────────────────────────

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Step 5: Tracking mode selection';
    RAISE NOTICE '========================================';
END $$;

ALTER TABLE agent_executions
ADD COLUMN selected_router_mode_id UUID REFERENCES tool_router_modes(id) ON DELETE SET NULL;

-- Index for analytics queries
CREATE INDEX idx_agent_executions_router_mode ON agent_executions(selected_router_mode_id);

DO $$
BEGIN
    RAISE NOTICE '✅ Added selected_router_mode_id column to agent_executions';
    RAISE NOTICE '   - Tracks which mode was selected for each execution';
    RAISE NOTICE '   - NULL = no router used (backward compatible)';
    RAISE NOTICE '   - Enables mode effectiveness analytics';
    RAISE NOTICE '   - Note: Old selected_mode_id column (agent_modes) still exists';
    RAISE NOTICE '✅ Created index: idx_agent_executions_router_mode';
END $$;

-- ────────────────────────────────────────────────────────────────────────────
-- STEP 6: Deprecate agent_modes Table
-- ────────────────────────────────────────────────────────────────────────────

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Step 6: Deprecating agent_modes';
    RAISE NOTICE '========================================';
END $$;

-- Mark table as deprecated (don't drop yet - migration happens in Phase 10)
COMMENT ON TABLE agent_modes IS
'DEPRECATED: Use tool_router_modes instead.
Migrate data via Phase 10 migration script.
Will be dropped after verification.
DO NOT add new agent_modes - use tool_router_modes.';

DO $$
BEGIN
    RAISE NOTICE '⚠️  Marked agent_modes table as DEPRECATED';
    RAISE NOTICE '   - Existing data preserved';
    RAISE NOTICE '   - Migration to tool_router_modes happens in Phase 10';
    RAISE NOTICE '   - Table will be dropped after verification';
END $$;

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
    modes_count INT;
    mode_tools_count INT;
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'VERIFICATION';
    RAISE NOTICE '========================================';

    -- Check tool_routers columns
    SELECT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'tool_routers' AND column_name = 'parent_router_id'
    ) INTO tool_routers_parent_exists;

    SELECT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'tool_routers' AND column_name = 'level'
    ) INTO tool_routers_level_exists;

    -- Check new tables
    SELECT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'tool_router_modes'
    ) INTO tool_router_modes_exists;

    SELECT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'tool_router_mode_tools'
    ) INTO tool_router_mode_tools_exists;

    -- Check agents column
    SELECT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'agents' AND column_name = 'router_id'
    ) INTO agents_router_id_exists;

    -- Check agent_executions column
    SELECT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'agent_executions' AND column_name = 'selected_router_mode_id'
    ) INTO executions_router_mode_id_exists;

    -- Get row counts
    SELECT COUNT(*) FROM tool_router_modes INTO modes_count;
    SELECT COUNT(*) FROM tool_router_mode_tools INTO mode_tools_count;

    -- Report results
    IF tool_routers_parent_exists AND tool_routers_level_exists THEN
        RAISE NOTICE '✅ tool_routers: parent_router_id and level columns added';
    ELSE
        RAISE WARNING '❌ tool_routers: missing columns!';
    END IF;

    IF tool_router_modes_exists THEN
        RAISE NOTICE '✅ tool_router_modes table created (% rows)', modes_count;
    ELSE
        RAISE WARNING '❌ tool_router_modes table missing!';
    END IF;

    IF tool_router_mode_tools_exists THEN
        RAISE NOTICE '✅ tool_router_mode_tools table created (% rows)', mode_tools_count;
    ELSE
        RAISE WARNING '❌ tool_router_mode_tools table missing!';
    END IF;

    IF agents_router_id_exists THEN
        RAISE NOTICE '✅ agents: router_id column added';
    ELSE
        RAISE WARNING '❌ agents: router_id column missing!';
    END IF;

    IF executions_router_mode_id_exists THEN
        RAISE NOTICE '✅ agent_executions: selected_router_mode_id column added';
    ELSE
        RAISE WARNING '❌ agent_executions: selected_router_mode_id column missing!';
    END IF;

    -- Final summary
    IF tool_routers_parent_exists AND
       tool_routers_level_exists AND
       tool_router_modes_exists AND
       tool_router_mode_tools_exists AND
       agents_router_id_exists AND
       executions_router_mode_id_exists THEN
        RAISE NOTICE '';
        RAISE NOTICE '========================================';
        RAISE NOTICE '✅ MIGRATION 064 SUCCESSFUL';
        RAISE NOTICE '========================================';
        RAISE NOTICE '';
        RAISE NOTICE 'Summary:';
        RAISE NOTICE '  - Extended tool_routers (parent_router_id, level)';
        RAISE NOTICE '  - Created tool_router_modes';
        RAISE NOTICE '  - Created tool_router_mode_tools';
        RAISE NOTICE '  - Linked agents → routers';
        RAISE NOTICE '  - Tracking mode selection in executions';
        RAISE NOTICE '  - Deprecated agent_modes';
        RAISE NOTICE '';
        RAISE NOTICE 'Ready for Phase 2: Rust database layer';
        RAISE NOTICE '';
    ELSE
        RAISE WARNING '';
        RAISE WARNING '========================================';
        RAISE WARNING '❌ MIGRATION 064 INCOMPLETE';
        RAISE WARNING '========================================';
        RAISE WARNING 'Some elements are missing! Review logs above.';
        RAISE WARNING '';
    END IF;
END $$;
