-- Remove old protocol types
-- The decomp, transform, review, route, and default protocols have been removed
-- from the codebase. This migration cleans up their database rows and tightens
-- the CHECK constraint to only allow 'documenter'.

-- 1. Delete old protocol rows (agents/schemas/templates left intact for FK safety)
DELETE FROM protocols WHERE protocol_type IN ('decomp', 'transform', 'review', 'route', 'default');

-- 2. Tighten CHECK constraint to documenter only
ALTER TABLE protocols DROP CONSTRAINT IF EXISTS protocols_type_check;
ALTER TABLE protocols ADD CONSTRAINT protocols_type_check
    CHECK (protocol_type = ANY (ARRAY['documenter']));
