-- Add metadata column to tasks table for routing hints and tracking
ALTER TABLE tasks ADD COLUMN metadata JSONB;
