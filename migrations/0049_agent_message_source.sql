-- Add source_type to chat_messages for agent-to-agent messaging.
-- NULL = human (backward compatible, no backfill needed).
-- 'agent' = injected by another agent via messaging service.
-- 'system' = system-generated notification.

ALTER TABLE public.chat_messages
  ADD COLUMN source_type TEXT;

ALTER TABLE public.chat_messages
  ADD CONSTRAINT chat_messages_source_type_check
  CHECK (source_type IS NULL OR source_type IN ('human', 'agent', 'system'));
