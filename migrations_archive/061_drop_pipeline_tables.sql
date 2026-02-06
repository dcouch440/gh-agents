-- Phase 4: Drop Pipeline Tables
-- This migration removes all pipeline-related tables from the database.
--
-- PREREQUISITES: Migration 060 must have been applied successfully.
-- Migration 060 drops all external foreign keys referencing pipeline tables.

-- ============================================================================
-- Step 1: Pre-Drop Foreign Key Verification (CRITICAL)
-- ============================================================================

-- Verify NO foreign keys reference pipeline tables.
-- This query MUST return zero rows or the table drop will fail.
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

    RAISE NOTICE 'Pre-drop FK verification: % foreign keys reference pipeline tables', fk_count;

    IF fk_count > 0 THEN
        RAISE EXCEPTION 'Cannot drop pipeline tables: % foreign key constraints still exist! Run migration 060 first.', fk_count;
    ELSE
        RAISE NOTICE '✅ No foreign keys reference pipeline tables - safe to proceed';
    END IF;
END $$;

-- ============================================================================
-- Step 2: Drop Pipeline Tables (in reverse dependency order)
-- ============================================================================

-- Drop tables that depend on other pipeline tables first,
-- then work backwards to the root tables.

-- Level 4: Side tasks (depends on stage_executions + pipeline_stages)
DROP TABLE IF EXISTS stage_side_tasks CASCADE;

-- Level 3: Stage members (depends on pipeline_stages)
DROP TABLE IF EXISTS pipeline_stage_members CASCADE;

-- Level 3: Stage executions (depends on pipeline_runs)
DROP TABLE IF EXISTS stage_executions CASCADE;

-- Level 2: Pipeline runs (depends on pipelines)
DROP TABLE IF EXISTS pipeline_runs CASCADE;

-- Level 2: Pipeline stages (depends on pipelines)
DROP TABLE IF EXISTS pipeline_stages CASCADE;

-- Level 1: Root pipeline table
DROP TABLE IF EXISTS pipelines CASCADE;

-- Log successful drops
DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Pipeline Tables Dropped Successfully';
    RAISE NOTICE '========================================';
    RAISE NOTICE '';
    RAISE NOTICE 'Dropped tables:';
    RAISE NOTICE '  ✅ stage_side_tasks';
    RAISE NOTICE '  ✅ pipeline_stage_members';
    RAISE NOTICE '  ✅ stage_executions';
    RAISE NOTICE '  ✅ pipeline_runs';
    RAISE NOTICE '  ✅ pipeline_stages';
    RAISE NOTICE '  ✅ pipelines';
    RAISE NOTICE '';
    RAISE NOTICE 'Pipeline system removal complete!';
    RAISE NOTICE '';
END $$;

-- ============================================================================
-- Final Verification
-- ============================================================================

-- Verify tables are gone
DO $$
DECLARE
    remaining_count INT;
BEGIN
    SELECT COUNT(*) INTO remaining_count
    FROM information_schema.tables
    WHERE table_schema = 'public'
        AND table_name IN (
            'pipelines',
            'pipeline_stages',
            'pipeline_runs',
            'stage_executions',
            'pipeline_stage_members',
            'stage_side_tasks'
        );

    IF remaining_count > 0 THEN
        RAISE WARNING 'Found % pipeline tables still present!', remaining_count;
    ELSE
        RAISE NOTICE '✅ All pipeline tables successfully removed';
    END IF;
END $$;
