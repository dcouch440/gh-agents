-- Bind chat sessions to pipelines for "powered chats"
ALTER TABLE chat_sessions ADD COLUMN pipeline_id UUID REFERENCES pipelines(id);
