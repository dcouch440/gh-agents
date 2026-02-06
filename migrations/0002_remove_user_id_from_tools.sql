-- ============================================================================
-- Migration 0002: Remove user_id from tools table
-- ============================================================================
-- Purpose: Remove user_id column from tools table as tools are system-wide
--          resources that require backend code to execute, not user-specific.
--
-- Changes:
-- - Drop foreign key constraint to users table
-- - Drop unique constraint on (user_id, name)
-- - Add unique constraint on name alone
-- - Drop index on user_id
-- - Drop user_id column
--
-- Date: 2026-02-05
-- ============================================================================

-- Drop foreign key constraint first
ALTER TABLE tools DROP CONSTRAINT IF EXISTS tools_user_id_fkey;

-- Drop unique constraint on (user_id, name)
ALTER TABLE tools DROP CONSTRAINT IF EXISTS tools_user_id_name_key;

-- Drop index on user_id
DROP INDEX IF EXISTS idx_tools_user;

-- Add unique constraint on name alone (tools are now system-wide)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'tools_name_key'
    ) THEN
        ALTER TABLE tools ADD CONSTRAINT tools_name_key UNIQUE (name);
    END IF;
END $$;

-- Finally, drop the user_id column if it exists
ALTER TABLE tools DROP COLUMN IF EXISTS user_id;
