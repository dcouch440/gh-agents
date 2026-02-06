-- Phase 1: Remove Pipeline Dependencies
-- This migration drops foreign key columns that reference pipeline tables,
-- preparing for pipeline table removal in migration 061.
--
-- CRITICAL: This migration MUST complete successfully before dropping pipeline tables,
-- otherwise the table drop will fail with FK constraint violations.

-- ============================================================================
-- Step 1: Pre-Migration Verification
-- ============================================================================

-- Check for existing data that will be affected
DO $$
DECLARE
    room_session_count INT;
    agent_execution_count INT;
    room_count INT;
    session_count INT;
BEGIN
    SELECT COUNT(*) INTO room_session_count FROM room_sessions WHERE run_id IS NOT NULL;
    SELECT COUNT(*) INTO agent_execution_count FROM agent_executions WHERE stage_execution_id IS NOT NULL;
    SELECT COUNT(*) INTO room_count FROM rooms WHERE pipeline_id IS NOT NULL;
    SELECT COUNT(*) INTO session_count FROM chat_sessions WHERE pipeline_id IS NOT NULL;

    RAISE NOTICE 'Pre-migration data count:';
    RAISE NOTICE '  - room_sessions with run_id: %', room_session_count;
    RAISE NOTICE '  - agent_executions with stage_execution_id: %', agent_execution_count;
    RAISE NOTICE '  - rooms with pipeline_id: %', room_count;
    RAISE NOTICE '  - chat_sessions with pipeline_id: %', session_count;
END $$;

-- ============================================================================
-- Step 2: Backup Existing Data
-- ============================================================================

-- Create backup tables for rollback capability
CREATE TABLE IF NOT EXISTS pipelines_backup AS
SELECT * FROM pipelines;

CREATE TABLE IF NOT EXISTS rooms_backup AS
SELECT * FROM rooms WHERE pipeline_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS room_sessions_backup AS
SELECT * FROM room_sessions WHERE run_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS agent_executions_backup AS
SELECT * FROM agent_executions WHERE stage_execution_id IS NOT NULL;

-- Log backup creation
DO $$
BEGIN
    RAISE NOTICE 'Backup tables created successfully';
    RAISE NOTICE '  - pipelines_backup';
    RAISE NOTICE '  - rooms_backup';
    RAISE NOTICE '  - room_sessions_backup';
    RAISE NOTICE '  - agent_executions_backup';
END $$;

-- ============================================================================
-- Step 3: Drop Foreign Key Dependencies (CRITICAL)
-- ============================================================================

-- Remove room_sessions → pipeline_runs link
-- This column tracks which pipeline run a room session belongs to.
-- After removal, room sessions become independent of pipeline execution.
ALTER TABLE room_sessions DROP COLUMN IF EXISTS run_id;

-- Remove agent_executions → stage_executions link
-- Make nullable first for safer rollback, then drop.
-- IMPORTANT: agent_executions table itself STAYS - it's core execution tracking.
-- We're only removing the FK field that links to pipeline stages.
ALTER TABLE agent_executions ALTER COLUMN stage_execution_id DROP NOT NULL;
ALTER TABLE agent_executions DROP COLUMN IF EXISTS stage_execution_id;

-- Log FK removal
DO $$
BEGIN
    RAISE NOTICE 'Foreign key columns dropped successfully';
    RAISE NOTICE '  - room_sessions.run_id removed';
    RAISE NOTICE '  - agent_executions.stage_execution_id removed';
END $$;

-- ============================================================================
-- Step 4: Migrate Rooms to Workflow Collections
-- ============================================================================

-- Replace pipeline_id with collection_id (clean migration to new system)
-- Rooms migrate from the deprecated pipelines table to the new workflow_collections table.
-- This is a direct replacement of the orchestration system.

-- Drop old pipeline reference
ALTER TABLE rooms DROP COLUMN IF EXISTS pipeline_id;

-- Add new collection reference
ALTER TABLE rooms ADD COLUMN IF NOT EXISTS collection_id UUID
    REFERENCES workflow_collections(id) ON DELETE SET NULL;

-- Note: Existing rooms will have collection_id = NULL after migration.
-- Users can manually associate rooms with collections if desired.

-- Log rooms migration
DO $$
BEGIN
    RAISE NOTICE 'Rooms migrated to workflow_collections';
    RAISE NOTICE '  - rooms.pipeline_id removed';
    RAISE NOTICE '  - rooms.collection_id added (nullable)';
END $$;

-- ============================================================================
-- Step 5: Update Chat Sessions Schema
-- ============================================================================

-- Sessions already handle NULL pipeline_id gracefully, so we can drop it entirely.
ALTER TABLE chat_sessions DROP COLUMN IF EXISTS pipeline_id;

-- Log session update
DO $$
BEGIN
    RAISE NOTICE 'Chat sessions updated';
    RAISE NOTICE '  - chat_sessions.pipeline_id removed';
END $$;

-- ============================================================================
-- Verification: Check No FKs Reference Pipeline Tables
-- ============================================================================

-- This query should return zero rows after this migration completes.
-- If any rows are returned, there are additional FK dependencies we missed.
DO $$
DECLARE
    fk_count INT;
BEGIN
    SELECT COUNT(*) INTO fk_count
    FROM information_schema.table_constraints AS tc
    JOIN information_schema.key_column_usage AS kcu
        ON tc.constraint_name = kcu.constraint_name
    JOIN information_schema.constraint_column_usage AS ccu
        ON ccu.constraint_name = tc.constraint_name
    WHERE tc.constraint_type = 'FOREIGN KEY'
        AND ccu.table_name IN (
            'pipelines',
            'pipeline_stages',
            'pipeline_runs',
            'stage_executions',
            'pipeline_stage_members',
            'stage_side_tasks'
        );

    RAISE NOTICE 'FK verification: % foreign keys still reference pipeline tables', fk_count;

    IF fk_count > 0 THEN
        RAISE WARNING 'Found % FK constraints still referencing pipeline tables!', fk_count;
        RAISE WARNING 'Migration may be incomplete. Run the verification query manually to see details.';
    ELSE
        RAISE NOTICE '✅ No foreign keys reference pipeline tables - safe to drop in migration 061';
    END IF;
END $$;

-- ============================================================================
-- Migration Complete
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Migration 060 Complete';
    RAISE NOTICE '========================================';
    RAISE NOTICE '';
    RAISE NOTICE 'Changes:';
    RAISE NOTICE '  ✅ Backed up affected tables';
    RAISE NOTICE '  ✅ Dropped room_sessions.run_id';
    RAISE NOTICE '  ✅ Dropped agent_executions.stage_execution_id';
    RAISE NOTICE '  ✅ Migrated rooms: pipeline_id → collection_id';
    RAISE NOTICE '  ✅ Dropped chat_sessions.pipeline_id';
    RAISE NOTICE '';
    RAISE NOTICE 'Next Steps:';
    RAISE NOTICE '  1. Phase 2: Remove backend pipeline code';
    RAISE NOTICE '  2. Phase 3: Remove frontend pipeline code';
    RAISE NOTICE '  3. Phase 4: Run migration 061 to drop pipeline tables';
    RAISE NOTICE '';
END $$;
