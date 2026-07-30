-- Soft-delete for conversation rebase: hide messages after a checkpoint
-- without duplicating or moving data.
ALTER TABLE chat_messages ADD COLUMN hidden_at TIMESTAMPTZ;

-- Partial index for fast filtering — only indexes hidden rows.
CREATE INDEX idx_cm_hidden ON chat_messages(session_id, hidden_at) WHERE hidden_at IS NOT NULL;
