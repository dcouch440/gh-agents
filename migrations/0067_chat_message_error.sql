-- Durable record of a failed chat turn.
--
-- Chat errors were previously pushed only to the live SSE stream and dropped
-- from the in-memory buffer 120s later, so a client that was not attached at
-- that instant lost the failure entirely and showed an indefinite spinner.
--
-- The error is attached to the *user* message that failed rather than a
-- synthetic assistant row: history building filters on role = 'assistant',
-- so a fake assistant turn would be replayed back to the model as context.
ALTER TABLE chat_messages ADD COLUMN error TEXT;
