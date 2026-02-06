-- Drop Cluster Tables
-- This migration removes all cluster-related tables and columns from the database.
--
-- Clusters were a deprecated system for organizing tools and agents into specialized groups.
-- This system has been superseded by the newer tool routing architecture.

-- ============================================================================
-- Step 1: Pre-Drop Verification
-- ============================================================================

-- Verify foreign keys referencing cluster tables
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
        AND ccu.table_name IN ('clusters', 'cluster_members');

    RAISE NOTICE 'Pre-drop verification: % foreign keys reference cluster tables', fk_count;

    IF fk_count > 2 THEN
        RAISE WARNING 'Found % foreign keys - expected only 2 (routing_events.cluster_id, cluster_members.cluster_id)', fk_count;
    END IF;
END $$;

-- ============================================================================
-- Step 2: Drop Tables (in reverse dependency order)
-- ============================================================================

-- Drop routing_events table (has FK to clusters)
DROP TABLE IF EXISTS routing_events CASCADE;

-- Drop cluster_members table (has FK to clusters)
DROP TABLE IF EXISTS cluster_members CASCADE;

-- Drop cluster_id column from tools table
ALTER TABLE tools DROP COLUMN IF EXISTS cluster_id;
DROP INDEX IF EXISTS idx_tools_cluster;

-- Drop clusters table (root table)
DROP TABLE IF EXISTS clusters CASCADE;

-- ============================================================================
-- Step 3: Final Verification
-- ============================================================================

DO $$
DECLARE
    remaining_count INT;
BEGIN
    SELECT COUNT(*) INTO remaining_count
    FROM information_schema.tables
    WHERE table_schema = 'public'
        AND table_name IN ('clusters', 'cluster_members', 'routing_events');

    IF remaining_count > 0 THEN
        RAISE WARNING 'Found % cluster tables still present!', remaining_count;
    ELSE
        RAISE NOTICE '✅ All cluster tables successfully removed';
    END IF;

    -- Verify cluster_id column removed from tools
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'tools' AND column_name = 'cluster_id'
    ) THEN
        RAISE WARNING 'tools.cluster_id column still exists!';
    ELSE
        RAISE NOTICE '✅ tools.cluster_id column successfully removed';
    END IF;
END $$;

-- Log completion
DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Cluster Tables Dropped Successfully';
    RAISE NOTICE '========================================';
    RAISE NOTICE '';
    RAISE NOTICE 'Dropped tables:';
    RAISE NOTICE '  ✅ routing_events';
    RAISE NOTICE '  ✅ cluster_members';
    RAISE NOTICE '  ✅ clusters';
    RAISE NOTICE '';
    RAISE NOTICE 'Dropped columns:';
    RAISE NOTICE '  ✅ tools.cluster_id';
    RAISE NOTICE '';
    RAISE NOTICE 'Cluster system removal complete!';
    RAISE NOTICE '';
END $$;
