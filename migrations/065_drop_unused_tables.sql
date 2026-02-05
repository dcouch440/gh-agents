-- Drop Unused Tables Migration
-- This migration removes 17 database tables that were created but never implemented.
--
-- These tables have ZERO active code references in the Rust codebase.
-- They exist only in their creation migrations and DATABASE_ERD.md documentation.
--
-- Tables being dropped:
--   - Task management: task_events, task_dependencies, cost_records, messages
--   - Project management: tickets, vertical_slices, prds, planning_sessions
--   - Automation: schedules, triggers
--   - Refactoring: refactor_sessions, refactor_changes
--   - Observability: decisions, llm_calls, token_usage
--   - Legacy: sessions (replaced by chat_sessions), system_state

-- ============================================================================
-- Step 1: Pre-Drop Foreign Key Verification (CRITICAL)
-- ============================================================================

-- Verify NO foreign keys from ACTIVE tables reference these unused tables.
-- This query checks for incoming foreign keys that would break active code.
-- Note: We exclude internal FKs between tables in the drop list (those are handled by CASCADE).
DO $$
DECLARE
    fk_count INT;
    tables_to_drop TEXT[] := ARRAY[
        'task_events',
        'task_dependencies',
        'cost_records',
        'tickets',
        'vertical_slices',
        'prds',
        'planning_sessions',
        'schedules',
        'triggers',
        'refactor_sessions',
        'refactor_changes',
        'decisions',
        'messages',
        'system_state',
        'llm_calls',
        'token_usage',
        'sessions'
    ];
BEGIN
    SELECT COUNT(*) INTO fk_count
    FROM information_schema.table_constraints AS tc
    JOIN information_schema.key_column_usage AS kcu
        ON tc.constraint_name = kcu.constraint_name
    JOIN information_schema.constraint_column_usage AS ccu
        ON ccu.constraint_name = tc.constraint_name
    WHERE tc.constraint_type = 'FOREIGN KEY'
        AND ccu.table_name = ANY(tables_to_drop)
        AND tc.table_name != ALL(tables_to_drop);  -- Exclude internal FKs

    RAISE NOTICE 'Pre-drop FK verification: % foreign keys from ACTIVE tables reference tables to be dropped', fk_count;

    IF fk_count > 0 THEN
        RAISE EXCEPTION 'Cannot drop tables: % foreign key constraints from active tables still exist!', fk_count;
    ELSE
        RAISE NOTICE '✅ No foreign keys from active tables - safe to proceed';
        RAISE NOTICE 'Note: Internal FKs between dropped tables (3) will be handled by CASCADE';
    END IF;
END $$;

-- ============================================================================
-- Step 2: Drop Tables (in reverse dependency order)
-- ============================================================================

-- Drop child tables first, then parent tables, then standalone tables.

-- ============================================================================
-- Group 1: Refactoring System
-- ============================================================================

-- refactor_changes depends on refactor_sessions
DROP TABLE IF EXISTS refactor_changes CASCADE;

-- refactor_sessions (parent table)
DROP TABLE IF EXISTS refactor_sessions CASCADE;

-- ============================================================================
-- Group 2: Project Management System
-- ============================================================================

-- planning_sessions depends on prds
DROP TABLE IF EXISTS planning_sessions CASCADE;

-- vertical_slices depends on tickets
DROP TABLE IF EXISTS vertical_slices CASCADE;

-- Parent tables
DROP TABLE IF EXISTS prds CASCADE;
DROP TABLE IF EXISTS tickets CASCADE;

-- ============================================================================
-- Group 3: Task Management Tables
-- ============================================================================

-- These reference tasks table (which is ACTIVE), but no active code queries them
DROP TABLE IF EXISTS task_events CASCADE;
DROP TABLE IF EXISTS task_dependencies CASCADE;

-- ============================================================================
-- Group 4: Observability & Cost Tracking
-- ============================================================================

-- Superseded by execution_messages, execution_variables, and token_ledger
DROP TABLE IF EXISTS decisions CASCADE;
DROP TABLE IF EXISTS llm_calls CASCADE;
DROP TABLE IF EXISTS cost_records CASCADE;

-- ============================================================================
-- Group 5: Automation System
-- ============================================================================

-- Scheduling and event triggers - never implemented
DROP TABLE IF EXISTS schedules CASCADE;
DROP TABLE IF EXISTS triggers CASCADE;

-- ============================================================================
-- Group 6: Token Tracking
-- ============================================================================

-- Superseded by token_ledger table
DROP TABLE IF EXISTS token_usage CASCADE;

-- ============================================================================
-- Group 7: Legacy Session Management
-- ============================================================================

-- Replaced by chat_sessions and room_sessions
DROP TABLE IF EXISTS sessions CASCADE;

-- ============================================================================
-- Group 8: Inter-Agent Messaging
-- ============================================================================

-- Never implemented - agents communicate through workflow execution system
DROP TABLE IF EXISTS messages CASCADE;

-- ============================================================================
-- Group 9: System State
-- ============================================================================

-- Production mode flag - never used
DROP TABLE IF EXISTS system_state CASCADE;

-- ============================================================================
-- Log Successful Drops
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Unused Tables Dropped Successfully';
    RAISE NOTICE '========================================';
    RAISE NOTICE '';
    RAISE NOTICE 'Refactoring System:';
    RAISE NOTICE '  ✅ refactor_changes';
    RAISE NOTICE '  ✅ refactor_sessions';
    RAISE NOTICE '';
    RAISE NOTICE 'Project Management:';
    RAISE NOTICE '  ✅ planning_sessions';
    RAISE NOTICE '  ✅ vertical_slices';
    RAISE NOTICE '  ✅ prds';
    RAISE NOTICE '  ✅ tickets';
    RAISE NOTICE '';
    RAISE NOTICE 'Task Management:';
    RAISE NOTICE '  ✅ task_events';
    RAISE NOTICE '  ✅ task_dependencies';
    RAISE NOTICE '';
    RAISE NOTICE 'Observability:';
    RAISE NOTICE '  ✅ decisions';
    RAISE NOTICE '  ✅ llm_calls';
    RAISE NOTICE '  ✅ cost_records';
    RAISE NOTICE '';
    RAISE NOTICE 'Automation:';
    RAISE NOTICE '  ✅ schedules';
    RAISE NOTICE '  ✅ triggers';
    RAISE NOTICE '';
    RAISE NOTICE 'Token Tracking:';
    RAISE NOTICE '  ✅ token_usage';
    RAISE NOTICE '';
    RAISE NOTICE 'Legacy:';
    RAISE NOTICE '  ✅ sessions';
    RAISE NOTICE '  ✅ messages';
    RAISE NOTICE '  ✅ system_state';
    RAISE NOTICE '';
    RAISE NOTICE 'Total tables dropped: 17';
    RAISE NOTICE 'Database cleanup complete!';
    RAISE NOTICE '';
END $$;

-- ============================================================================
-- Step 3: Final Verification
-- ============================================================================

-- Verify all tables are gone
DO $$
DECLARE
    remaining_count INT;
BEGIN
    SELECT COUNT(*) INTO remaining_count
    FROM information_schema.tables
    WHERE table_schema = 'public'
        AND table_name IN (
            'task_events',
            'task_dependencies',
            'cost_records',
            'tickets',
            'vertical_slices',
            'prds',
            'planning_sessions',
            'schedules',
            'triggers',
            'refactor_sessions',
            'refactor_changes',
            'decisions',
            'messages',
            'system_state',
            'llm_calls',
            'token_usage',
            'sessions'
        );

    IF remaining_count > 0 THEN
        RAISE WARNING 'Found % tables still present!', remaining_count;
    ELSE
        RAISE NOTICE '✅ All unused tables successfully removed';
    END IF;
END $$;
