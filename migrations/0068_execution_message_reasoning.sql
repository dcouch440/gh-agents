-- Capture the model's reasoning trace on execution messages.
--
-- DeepInfra's OpenAI-compatible API returns a `reasoning_content` field
-- (message.reasoning_content non-streaming, delta.reasoning_content while
-- streaming) alongside `content` for reasoning models. It was previously
-- parsed nowhere — serde silently dropped it — so no reasoning was ever
-- persisted, only the final answer.
ALTER TABLE execution_messages ADD COLUMN reasoning TEXT;
