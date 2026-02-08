-- Make protocols system-owned (not user-owned) and add 'default' protocol type.
-- Protocols are framework-level constructs, not user-created content.

-- Drop user_id column (FK, index, column)
ALTER TABLE protocols DROP CONSTRAINT IF EXISTS protocols_user_id_fkey;
DROP INDEX IF EXISTS idx_protocols_user_id;
ALTER TABLE protocols DROP COLUMN IF EXISTS user_id;

-- Update protocol_type CHECK to include 'default'
ALTER TABLE protocols DROP CONSTRAINT IF EXISTS protocols_type_check;
ALTER TABLE protocols ADD CONSTRAINT protocols_type_check CHECK (
    protocol_type IN ('decomp', 'transform', 'review', 'route', 'default')
);

-- Add UNIQUE on name for idempotent seeding (ON CONFLICT (name) DO NOTHING)
ALTER TABLE protocols ADD CONSTRAINT protocols_name_key UNIQUE (name);
