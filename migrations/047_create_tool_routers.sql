-- Tool routers: configurable LLM-based routers that own subsets of tools
CREATE TABLE tool_routers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT,
    system_prompt TEXT NOT NULL,
    model_id TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_tool_routers_user ON tool_routers(user_id);

-- Join table: which tools belong to which router
CREATE TABLE tool_router_tools (
    router_id UUID NOT NULL REFERENCES tool_routers(id) ON DELETE CASCADE,
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (router_id, tool_id)
);
CREATE INDEX idx_tool_router_tools_tool ON tool_router_tools(tool_id);
