-- ============================================================================
-- Migration 068: Tool Capability Registry
-- ============================================================================
-- Purpose: Create semantic capability taxonomy for tools.
--          Enable mode-based tool selection by capabilities rather than
--          explicit tool lists.
--
-- New Tables:
--   - tool_capabilities: Capability taxonomy (file_read, code_execution, etc.)
--   - tool_capability_assignments: Which capabilities each tool provides
--   - mode_required_capabilities: Which capabilities each mode requires
--
-- Seed Data:
--   - 15+ common capabilities
--   - Capability assignments for existing tools
-- ============================================================================

-- ============================================================================
-- 1. CREATE TABLES
-- ============================================================================

-- Capability taxonomy (predefined by system)
CREATE TABLE tool_capabilities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    capability_key TEXT NOT NULL UNIQUE,  -- Snake_case identifier (e.g., "file_read", "shell_execution")
    display_name TEXT NOT NULL,
    category TEXT NOT NULL,  -- "filesystem", "web", "computation", "version_control", "development", etc.
    safety_level TEXT NOT NULL DEFAULT 'safe',  -- "safe", "caution", "unsafe"
    description TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (capability_key ~ '^[a-z][a-z0-9_]*$')
);

CREATE INDEX idx_tool_capabilities_category ON tool_capabilities(category);
CREATE INDEX idx_tool_capabilities_safety ON tool_capabilities(safety_level);

COMMENT ON TABLE tool_capabilities IS
    'Semantic capability taxonomy for tools. Enables mode-based tool selection by required capabilities.';

-- Which capabilities each tool provides
CREATE TABLE tool_capability_assignments (
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    capability_id UUID NOT NULL REFERENCES tool_capabilities(id) ON DELETE CASCADE,
    PRIMARY KEY (tool_id, capability_id)
);

CREATE INDEX idx_tool_capability_assignments_capability ON tool_capability_assignments(capability_id);

COMMENT ON TABLE tool_capability_assignments IS
    'Maps tools to the capabilities they provide. A tool can provide multiple capabilities.';

-- Mode capability requirements (extends tool_router_modes)
CREATE TABLE mode_required_capabilities (
    mode_id UUID NOT NULL REFERENCES tool_router_modes(id) ON DELETE CASCADE,
    capability_id UUID NOT NULL REFERENCES tool_capabilities(id) ON DELETE CASCADE,
    is_required BOOLEAN NOT NULL DEFAULT true,
    PRIMARY KEY (mode_id, capability_id)
);

CREATE INDEX idx_mode_required_capabilities_mode ON mode_required_capabilities(mode_id);

COMMENT ON TABLE mode_required_capabilities IS
    'Defines which capabilities a mode requires. Mode resolver will auto-select tools providing these capabilities.';

-- ============================================================================
-- 2. SEED CAPABILITIES
-- ============================================================================

INSERT INTO tool_capabilities (capability_key, display_name, category, safety_level, description) VALUES
    ('file_read', 'File Reading', 'filesystem', 'safe', 'Read file contents from disk'),
    ('file_write', 'File Writing', 'filesystem', 'caution', 'Create or modify files'),
    ('file_search', 'File Search', 'filesystem', 'safe', 'Search for files by pattern'),
    ('content_search', 'Content Search', 'filesystem', 'safe', 'Search file contents (grep-like)'),
    ('git_read', 'Git Read Operations', 'version_control', 'safe', 'View git history, diffs, status'),
    ('git_write', 'Git Write Operations', 'version_control', 'caution', 'Commit, branch, merge, push'),
    ('shell_execution', 'Shell Execution', 'system', 'unsafe', 'Execute arbitrary shell commands'),
    ('web_fetch', 'Web Fetching', 'web', 'safe', 'Fetch content from URLs'),
    ('web_search', 'Web Search', 'web', 'safe', 'Search the internet'),
    ('code_analysis', 'Code Analysis', 'development', 'safe', 'Analyze code structure and quality'),
    ('test_execution', 'Test Execution', 'development', 'caution', 'Run test suites'),
    ('database_query', 'Database Query', 'data', 'caution', 'Query databases'),
    ('api_call', 'API Calls', 'integration', 'caution', 'Make HTTP API requests'),
    ('document_create', 'Document Creation', 'knowledge', 'safe', 'Create knowledge documents'),
    ('document_search', 'Document Search', 'knowledge', 'safe', 'Search knowledge documents')
ON CONFLICT (capability_key) DO NOTHING;

-- ============================================================================
-- 3. ASSIGN CAPABILITIES TO EXISTING TOOLS
-- ============================================================================
-- Based on current tool inventory: read_file, edit_file, write_file, list_files,
-- git_status, git_diff, git_branch, git_add, git_commit, run_command,
-- run_tests, web_research

-- File operations: read_file → file_read
INSERT INTO tool_capability_assignments (tool_id, capability_id)
SELECT t.id, c.id
FROM tools t
CROSS JOIN tool_capabilities c
WHERE t.name = 'read_file' AND c.capability_key = 'file_read'
ON CONFLICT DO NOTHING;

-- File operations: edit_file, write_file → file_write
INSERT INTO tool_capability_assignments (tool_id, capability_id)
SELECT t.id, c.id
FROM tools t
CROSS JOIN tool_capabilities c
WHERE t.name IN ('edit_file', 'write_file') AND c.capability_key = 'file_write'
ON CONFLICT DO NOTHING;

-- File operations: list_files → file_search
INSERT INTO tool_capability_assignments (tool_id, capability_id)
SELECT t.id, c.id
FROM tools t
CROSS JOIN tool_capabilities c
WHERE t.name = 'list_files' AND c.capability_key = 'file_search'
ON CONFLICT DO NOTHING;

-- Git operations: git_status, git_diff, git_branch (read) → git_read
INSERT INTO tool_capability_assignments (tool_id, capability_id)
SELECT t.id, c.id
FROM tools t
CROSS JOIN tool_capabilities c
WHERE t.name IN ('git_status', 'git_diff', 'git_branch') AND c.capability_key = 'git_read'
ON CONFLICT DO NOTHING;

-- Git operations: git_add, git_commit, git_branch (write) → git_write
-- Note: git_branch provides both git_read and git_write capabilities
INSERT INTO tool_capability_assignments (tool_id, capability_id)
SELECT t.id, c.id
FROM tools t
CROSS JOIN tool_capabilities c
WHERE t.name IN ('git_add', 'git_commit', 'git_branch') AND c.capability_key = 'git_write'
ON CONFLICT DO NOTHING;

-- System operations: run_command → shell_execution
INSERT INTO tool_capability_assignments (tool_id, capability_id)
SELECT t.id, c.id
FROM tools t
CROSS JOIN tool_capabilities c
WHERE t.name = 'run_command' AND c.capability_key = 'shell_execution'
ON CONFLICT DO NOTHING;

-- Test operations: run_tests → test_execution
INSERT INTO tool_capability_assignments (tool_id, capability_id)
SELECT t.id, c.id
FROM tools t
CROSS JOIN tool_capabilities c
WHERE t.name = 'run_tests' AND c.capability_key = 'test_execution'
ON CONFLICT DO NOTHING;

-- Web operations: web_research → web_search
INSERT INTO tool_capability_assignments (tool_id, capability_id)
SELECT t.id, c.id
FROM tools t
CROSS JOIN tool_capabilities c
WHERE t.name = 'web_research' AND c.capability_key = 'web_search'
ON CONFLICT DO NOTHING;
