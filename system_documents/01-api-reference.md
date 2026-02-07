# Nexor API Reference

Complete reference for all REST API endpoints, routes, request/response types, and business logic.

---

## Route Registration

All routes are registered in `/src/server/api/mod.rs` and prefixed with `/api/`. Authentication is JWT-based via `AuthUser` extractor (Bearer token or `?token=` query param). Most routes require auth except `/api/auth/setup`, `/api/auth/login`, `/api/health`, and `/api/config` GET.

---

## 1. Agents

**Module:** `src/server/api/agents/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/agents` | 200 | List all agents with pool stats |
| POST | `/api/agents` | 201 | Create a new agent |
| GET | `/api/agents/{id}` | 200 | Get single agent by UUID |
| PATCH | `/api/agents/{id}` | 200 | Partial update agent |
| DELETE | `/api/agents/{id}` | 204 | Delete agent |

### CreateAgentRequest
```json
{
  "name": "string (required, max MAX_TITLE_LENGTH)",
  "system_prompt": "string (max MAX_PROMPT_LENGTH)",
  "persona_style": "string (default: 'casual')",
  "model_provider": "string (default: 'anthropic')",
  "model_id": "string (required, non-empty)",
  "model_max_tokens": "int (default: 4096)",
  "model_temperature": "float (default: 0.7)",
  "output_schema_id": "uuid (optional)"
}
```

### UpdateAgentRequest
All fields optional, plus `router_id: uuid`.

### AgentsListResponse
```json
{
  "agents": [AgentResponse],
  "pool_stats": { "total": int, "available": int, "max": int }
}
```

### AgentResponse
```json
{
  "id": "uuid",
  "name": "string",
  "system_prompt": "string",
  "persona_style": "string",
  "model_provider": "string",
  "model_id": "string",
  "model_max_tokens": int,
  "model_temperature": float,
  "output_schema_id": "uuid|null",
  "router_id": "uuid|null",
  "status": "string",
  "version": int
}
```

**Business Logic:**
- Validates `model_id` not empty, name/prompt length limits
- Default values: `persona_style="casual"`, `model_provider="anthropic"`, `temperature=0.7`, `max_tokens=4096`, `status="idle"`
- Verifies agent ownership before get/update/delete via `verify_agent_ownership`

---

## 2. Agent Context

**Module:** `src/server/api/agent_context/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/agents/{id}/context` | 200 | Get context documents for agent |
| PUT | `/api/agents/{id}/context` | 200 | Set/replace context documents |

### SetAgentContextRequest
```json
{ "document_ids": ["uuid-string", ...] }
```

### AgentContextResponse
```json
{
  "agent_id": "string",
  "documents": [{
    "id": "uuid", "title": "string", "summary": "string|null",
    "ref_tag": "string", "tags": ["string"], "doc_type": "string",
    "updated_at": "datetime"
  }]
}
```

**Business Logic:** PUT = full replace of all context document associations.

---

## 3. Agent Executions

**Module:** `src/server/api/agent_executions/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/agent-executions` | 200 | List executions (optional `?status=` filter) |
| GET | `/api/agent-executions/{id}` | 200 | Get single execution |
| GET | `/api/agent-executions/{id}/messages` | 200 | List execution messages |
| POST | `/api/agent-executions/{id}/messages` | 202 | Send message to interactive execution |
| GET | `/api/agent-executions/{id}/messages/{stream_id}/stream` | SSE | Stream agent response |
| POST | `/api/agent-executions/{id}/approve` | 200 | Approve/complete interactive execution |
| PUT | `/api/agent-executions/{id}/exemplary` | 200 | Mark execution as exemplary |

### AgentExecutionResponse
```json
{
  "id": "uuid",
  "agent_id": "uuid",
  "workflow_step_id": "uuid|null",
  "is_interactive": bool,
  "parent_agent_execution_id": "uuid|null",
  "system_prompt_rendered": "string",
  "input": "string",
  "output": "string|null",
  "structured_output": "json|null",
  "selected_mode_id": "uuid|null",
  "status": "string",
  "started_at": "datetime",
  "completed_at": "datetime|null",
  "is_exemplary": bool
}
```

### SendMessageRequest / SendMessageResponse
```json
// Request
{ "content": "string" }

// Response (202)
{
  "message": { ExecutionMessageResponse },
  "stream_id": "uuid"
}
```

### ApproveExecutionRequest
```json
{ "structured_output": "json (optional)" }
```

**Business Logic:**
- `send_execution_message`: Validates `is_interactive` and `status="awaiting_user"`. Records user message, spawns background LLM call via `hub::run_interactive_chat`, returns 202 with `stream_id`.
- `execution_message_stream`: SSE endpoint streaming tokens, tool events, doc updates.
- `approve_execution`: Marks as "completed", optionally updates `structured_output`. Checks if all interactive reviews are complete, resumes DAG via `hub::dag::resume_dag_from_approval`.
- `set_exemplary`: Marks execution for few-shot learning in future runs.

---

## 4. Auth

**Module:** `src/server/api/auth/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| POST | `/api/auth/setup` | 200/409 | First-run password setup |
| POST | `/api/auth/register` | 201 | Register new user |
| POST | `/api/auth/login` | 200 | Authenticate, get JWT |
| GET | `/api/auth/me` | 200 | Get current user from token |

### RegisterRequest / LoginRequest
```json
{ "email": "string", "password": "string" }
```

### AuthTokenResponse (register)
```json
{ "token": "string", "expires_in": int, "user": { "id": "uuid", "email": "string", "github_login": "string|null" } }
```

### LoginResponse
```json
{ "token": "string", "expires_in": int }
```

### MeResponse
```json
{ "id": "uuid", "email": "string", "github_login": "string|null", "is_admin": bool, "authenticated": true, "token_expires": "datetime" }
```

**Business Logic:**
- `auth_setup`: Password >= 8 chars. Returns 409 if already configured.
- `auth_register`: Validates email format, checks duplicates (409). Seeds built-in tools on first user.
- `auth_login`: Returns 401 on mismatch. Creates 24-hour JWT.
- JWT signed with HS256 using `NEXOR_JWT_SECRET` env var.

---

## 5. Cancellation

**Module:** `src/server/api/cancellation/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| POST | `/agent-executions/{execution_id}/cancel` | 200 | Cancel execution |

Returns `{ "status": "cancelled" }`. Returns 404 if not found.

---

## 6. Chat

**Module:** `src/server/api/chat/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| POST | `/api/chat` | 202 | Send chat message |
| GET | `/api/chat/history` | 200 | Get chat history (paginated) |
| GET | `/api/chat/{message_id}/stream` | SSE | Stream chat response |
| GET | `/api/sessions/{session_id}/chat/{message_id}/stream` | SSE | Session-scoped chat stream |
| DELETE | `/api/chat/history` | 204 | Clear all chat history |

### ChatRequest / ChatResponse
```json
// Request
{ "message": "string" }

// Response (202)
{ "message_id": "uuid", "status": "queued" }
```

### HistoryQuery
`?limit=50&offset=0`

### SSE Event Types
- `token` - Text token
- `tool_start` - Tool execution started (`{ name, tool_id }`)
- `tool_end` - Tool execution finished (`{ name, tool_id }`)
- `doc_update` - Document created/updated (`{ doc_id, title }`)
- `done` - Stream completed
- `error` - Stream error

**Business Logic:**
- `send_chat`: Validates non-empty + length. Pre-creates buffered stream. Stores user message in DB. Queues `ConsumerMessage` to `chat_tx` channel. Returns 202.
- `chat_stream`: SSE endpoint that replays buffered chunks before connecting to live broadcast channel.

---

## 7. Collections

**Module:** `src/server/api/collections/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/collections` | 200 | List collections |
| POST | `/api/collections` | 201 | Create collection |
| GET | `/api/collections/{id}` | 200 | Get collection |
| PUT | `/api/collections/{id}` | 200 | Update collection |
| DELETE | `/api/collections/{id}` | 204 | Delete collection |
| POST | `/api/collections/{id}/run` | 202 | Execute collection |
| GET | `/api/collections/runs/{run_id}/status` | 200 | Get run status |

### CreateCollectionRequest
```json
{
  "name": "string",
  "description": "string",
  "execution_mode": "sequential|parallel (default: parallel)"
}
```

### CollectionRunResponse
```json
{
  "id": "uuid", "collection_id": "uuid", "user_id": "uuid",
  "status": "string", "started_at": "datetime",
  "completed_at": "datetime|null", "error": "string|null"
}
```

**Business Logic:** `run_collection` spawns `CollectionDagExecutor`, returns 202 with run status.

---

## 8. Config

**Module:** `src/server/api/config/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/config` | 200 | Get current configuration |
| PATCH | `/api/config` | 200 | Update configuration |

### ConfigResponse
```json
{
  "verbosity": "quiet|normal|verbose",
  "pool": { "max_agents": int },
  "autonomy": "full_auto|approval_gates|supervised",
  "git_strategy": "branch_per_slice|branch_per_ticket",
  "sandbox_mode": "docker|local_restricted|none"
}
```

**No authentication required.**

---

## 9. Costs

**Module:** `src/server/api/costs/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/costs` | 200 | Get cost breakdown |

### CostQuery
`?since=2024-01-01T00:00:00Z` (optional DateTime filter)

### CostResponse
```json
{
  "total_spend": 12.50,
  "models": [{ "model_id": "string", "input_tokens": int, "output_tokens": int, "cost_usd": float }]
}
```

---

## 10. Documents

**Module:** `src/server/api/documents/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/documents` | 200 | List documents |
| GET | `/api/documents/search?q=query` | 200 | Search documents |
| GET | `/api/documents/{id}` | 200 | Get single document |
| POST | `/api/documents` | 201 | Create document |
| PATCH | `/api/documents/{id}` | 200 | Update document |
| DELETE | `/api/documents/{id}` | 204 | Delete document |

### CreateDocumentRequest
```json
{
  "title": "string (required, max MAX_TITLE_LENGTH)",
  "content": "string (max MAX_DESCRIPTION_LENGTH)",
  "doc_type": "string (default: 'architecture')",
  "session_id": "uuid (optional)",
  "tags": ["string"]
}
```

### DocumentResponse
```json
{
  "id": "uuid", "title": "string", "content": "string",
  "summary": "string|null", "ref_tag": "string", "tags": ["string"],
  "doc_type": "string", "session_id": "uuid|null",
  "created_at": "datetime", "updated_at": "datetime"
}
```

---

## 11. Output Schemas

**Module:** `src/server/api/output_schemas/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/output-schemas` | 200 | List output schemas |
| POST | `/api/output-schemas` | 201 | Create schema |
| GET | `/api/output-schemas/{id}` | 200 | Get schema |
| PUT | `/api/output-schemas/{id}` | 200 | Update schema |
| DELETE | `/api/output-schemas/{id}` | 204 | Delete schema |

### CreateOutputSchemaRequest
```json
{ "name": "string", "schema": { /* JSON Schema */ } }
```

---

## 12. Prompt Templates

**Module:** `src/server/api/prompt_templates/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/prompt-templates` | 200 | List templates |
| POST | `/api/prompt-templates` | 201 | Create template |
| GET | `/api/prompt-templates/{id}` | 200 | Get template |
| PUT | `/api/prompt-templates/{id}` | 200 | Update template |
| DELETE | `/api/prompt-templates/{id}` | 204 | Delete template |

### CreatePromptTemplateRequest
```json
{
  "name": "string (required, max MAX_TITLE_LENGTH)",
  "content": "string (max MAX_PROMPT_LENGTH)"
}
```

---

## 13. Results

**Module:** `src/server/api/results/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/results` | 200 | List results (optional `?output_schema_id=` filter) |
| GET | `/api/results/{id}` | 200 | Get single result |
| DELETE | `/api/results/{id}` | 204 | Delete result |

### ResultResponse
```json
{
  "id": "uuid", "agent_execution_id": "uuid",
  "output_schema_id": "uuid|null", "name": "string",
  "data": { /* JSON */ }, "created_at": "datetime"
}
```

---

## 14. Rooms

**Module:** `src/server/api/rooms/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| POST | `/api/rooms` | 201 | Create room |
| GET | `/api/rooms/{id}` | 200 | Get room |
| PUT | `/api/rooms/{id}` | 200 | Update room |
| DELETE | `/api/rooms/{id}` | 204 | Delete room |
| GET | `/api/rooms/{id}/members` | 200 | List room members |
| POST | `/api/rooms/{id}/members` | 201 | Add room member |
| DELETE | `/api/rooms/{id}/members/{agent_id}` | 204 | Remove member |
| PUT | `/api/rooms/{id}/members` | 200 | Set all members (replace) |
| POST | `/api/rooms/{id}/sessions` | 201 | Start room session |
| GET | `/api/room-sessions/{id}` | 200 | Get session |
| POST | `/api/room-sessions/{id}/messages` | 202 | Send message to session |
| GET | `/api/room-sessions/{id}/transcript` | 200 | Get session transcript |
| POST | `/api/room-sessions/{id}/close` | 204 | Close session |
| GET | `/api/room-sessions/{id}/outputs` | 200 | List structured outputs |

### CreateRoomRequest
```json
{
  "collection_id": "uuid (optional)",
  "name": "string",
  "gatekeeper_enabled": bool,
  "gatekeeper_model_id": "string (default: 'claude-haiku-4-20250414')",
  "max_speakers_per_turn": 4,
  "max_turns": 20,
  "tools_enabled": bool
}
```

### AddRoomMemberRequest
```json
{
  "agent_id": "uuid",
  "display_name": "string (optional)",
  "role_description": "string",
  "display_order": int
}
```

### RoomMessageRequest
```json
{ "content": "string" }
```

### RoomOutputResponse
```json
{
  "id": "uuid", "agent_id": "uuid", "speaker_order": int,
  "turn_number": int, "output_name": "string",
  "structured_output": { /* JSON */ }, "raw_output": "string"
}
```

**Business Logic:**
- `send_room_message`: Validates non-empty. Loads session/room/members/agents. Creates LLM provider. Spawns background `execute_room_turn`. Returns 200 with `{ "status": "processing", "session_id" }`.
- `close_room_session`: Returns 409 if already completed. Updates status to "completed". If DAG-linked, extracts outputs and resumes workflow via `hub::dag::resume_dag_from_approval`.

---

## 15. Router Modes

**Module:** `src/server/api/router_modes/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/tool-routers/{router_id}/modes` | 200 | List modes for router |
| POST | `/api/tool-routers/{router_id}/modes` | 201 | Create mode |
| GET | `/api/tool-routers/{router_id}/modes/{id}` | 200 | Get mode |
| PUT | `/api/tool-routers/{router_id}/modes/{id}` | 200 | Update mode |
| DELETE | `/api/tool-routers/{router_id}/modes/{id}` | 204 | Delete mode |
| PUT | `/api/tool-routers/{router_id}/modes/{id}/tools` | 200 | Set mode tools |

### CreateRouterModeRequest
```json
{
  "mode_key": "string (snake_case, regex: ^[a-z][a-z0-9_]*$, 1-50 chars)",
  "display_name": "string",
  "description": "string",
  "system_prompt": "string",
  "temperature": 0.7,
  "max_tokens": 4096,
  "append_to_agent_system_prompt": bool,
  "append_to_agent_tools": bool,
  "display_order": int
}
```

**Validation:** `temperature: 0.0-2.0`, `max_tokens: 1-200,000`.

---

## 16. Routing Rules

**Module:** `src/server/api/routing_rules/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/workflows/{wid}/steps/{sid}/routing-rules` | 200 | List rules |
| POST | `/api/workflows/{wid}/steps/{sid}/routing-rules` | 201 | Create rule |
| PUT | `/api/workflows/{wid}/steps/{sid}/routing-rules/{id}` | 200 | Update rule |
| DELETE | `/api/workflows/{wid}/steps/{sid}/routing-rules/{id}` | 204 | Delete rule |

### CreateRoutingRuleRequest
```json
{
  "label_value": "string",
  "agent_id": "uuid",
  "description": "string (optional)",
  "display_order": int
}
```

**Business Logic:** `verify_step_access` verifies workflow ownership and step membership. Returns 404 if not found/owned/step not in workflow.

---

## 17. Session Context

**Module:** `src/server/api/session_context/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/sessions/{session_id}/context` | 200 | Get context entries (limit 100) |
| GET | `/api/sessions/{session_id}/requests` | 200 | List router requests |

Returns raw `Vec<ContextStoreRow>` and `Vec<RouterRequestRow>`.

---

## 18. Sessions

**Module:** `src/server/api/sessions/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/modes` | 200 | List available modes (agents) |
| GET | `/api/agents/{agent_id}/modes` | 200 | List agent modes |
| POST | `/api/agents/{agent_id}/modes` | 201 | Create agent mode |

### AgentModeResponse
```json
{
  "id": "uuid", "agent_id": "uuid", "name": "string",
  "system_prompt_suffix": "string|null",
  "temperature_override": "float|null",
  "model_override": "string|null",
  "tool_overrides": ["string"]|null,
  "classifier_hint": "string",
  "created_at": "datetime", "version": int
}
```

---

## 19. Step Ports

**Module:** `src/server/api/step_ports/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/workflows/{wid}/steps/{sid}/inputs` | 200 | List input ports |
| POST | `/api/workflows/{wid}/steps/{sid}/inputs` | 201 | Create input port |
| DELETE | `/api/workflows/{wid}/steps/{sid}/inputs/{pid}` | 204 | Delete input |
| GET | `/api/workflows/{wid}/steps/{sid}/outputs` | 200 | List output ports |
| POST | `/api/workflows/{wid}/steps/{sid}/outputs` | 201 | Create output port |
| DELETE | `/api/workflows/{wid}/steps/{sid}/outputs/{pid}` | 204 | Delete output |

### CreateStepInputRequest
```json
{
  "port_name": "string (required)",
  "port_type": "string (default: 'string')",
  "required": bool,
  "default_value": "json|null",
  "description": "string|null",
  "json_schema": "json|null"
}
```

### CreateStepOutputRequest
```json
{
  "port_name": "string (required)",
  "port_type": "string (default: 'string')",
  "json_path": "string (required, non-empty)",
  "description": "string|null",
  "json_schema": "json|null"
}
```

---

## 20. Tasks

**Module:** `src/server/api/tasks/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/tasks` | 200 | List tasks (`?status=&limit=`) |
| GET | `/api/tasks/{id}` | 200 | Get single task |
| POST | `/api/tasks` | 201 | Create task |

### CreateTaskRequest
```json
{
  "title": "string (required, max MAX_TITLE_LENGTH)",
  "description": "string (optional, max MAX_DESCRIPTION_LENGTH)",
  "priority": "low|normal|high|urgent (default: normal)",
  "tier": "string (optional)"
}
```

---

## 21. Tool Routers

**Module:** `src/server/api/tool_routers/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/tool-routers` | 200 | List routers |
| POST | `/api/tool-routers` | 201 | Create router |
| GET | `/api/tool-routers/{id}` | 200 | Get router |
| PUT | `/api/tool-routers/{id}` | 200 | Update router |

### CreateToolRouterRequest
```json
{
  "name": "string (required, max MAX_TITLE_LENGTH)",
  "description": "string",
  "system_prompt": "string",
  "model_id": "string"
}
```

---

## 22. Tools

**Module:** `src/server/api/tools/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/tools` | 200 | List all tools |
| POST | `/api/tools` | 201 | Create tool (admin only) |
| GET | `/api/tools/{id}` | 200 | Get tool |

### ToolResponse
```json
{
  "id": "uuid", "name": "string", "display_name": "string",
  "description": "string", "parameters": { /* JSON Schema */ },
  "version": int
}
```

**Business Logic:** `create_tool` requires admin via `require_admin(&auth)`. Tools are system-wide (not user-scoped).

---

## 23. Workflows

**Module:** `src/server/api/workflows/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/workflows` | 200 | List workflows |
| POST | `/api/workflows` | 201 | Create workflow |
| GET | `/api/workflows/{id}` | 200 | Get workflow |
| PUT | `/api/workflows/{id}` | 200 | Update workflow |
| DELETE | `/api/workflows/{id}` | 204 | Delete workflow |
| GET | `/api/workflows/{wid}/steps` | 200 | List steps |
| POST | `/api/workflows/{wid}/steps` | 201 | Create step |
| GET | `/api/workflows/{wid}/steps/{sid}` | 200 | Get step |
| PUT | `/api/workflows/{wid}/steps/{sid}` | 200 | Update step |
| DELETE | `/api/workflows/{wid}/steps/{sid}` | 204 | Delete step |
| GET | `/api/workflows/{wid}/edges` | 200 | List edges |
| POST | `/api/workflows/{wid}/edges` | 201 | Create edge |
| DELETE | `/api/workflows/{wid}/edges` | 204 | Delete edge |
| GET | `/api/workflows/{wid}/steps/{sid}/documents` | 200 | List step documents |
| POST | `/api/workflows/{wid}/steps/{sid}/documents` | 201 | Attach document |
| DELETE | `/api/workflows/{wid}/steps/{sid}/documents/{doc_id}` | 204 | Detach document |

### CreateWorkflowRequest
```json
{
  "name": "string (required)",
  "description": "string (optional)"
}
```

### UpdateWorkflowRequest
```json
{
  "name": "string (optional)",
  "description": "string (optional)",
  "container_enabled": bool,
  "target_repo_url": "string|null",
  "target_branch": "string|null",
  "vpn_enabled": bool
}
```

### WorkflowStepResponse
```json
{
  "id": "uuid",
  "workflow_id": "uuid",
  "agent_id": "uuid",
  "execution_mode": "single|for_each|cavernous|room",
  "for_each_ref": "string|null",
  "prompt_template_id": "uuid|null",
  "prompt_template": "string",
  "output_schema_id": "uuid|null",
  "output_variable_name": "string|null",
  "interactive_agent_id": "uuid|null",
  "for_each_label_field": "string|null",
  "display_order": int,
  "version": int,
  "reasoning_trace": bool,
  "verification_agent_ids": ["uuid"]|null
}
```

### CreateStepRequest
```json
{
  "agent_id": "uuid (required)",
  "execution_mode": "string (optional)",
  "for_each_ref": "string (optional)",
  "prompt_template_id": "uuid (optional)",
  "prompt_template": "string (optional)",
  "output_schema_id": "uuid (optional)",
  "output_variable_name": "string (optional)",
  "interactive_agent_id": "uuid (optional)",
  "for_each_label_field": "string (optional)",
  "display_order": int,
  "reasoning_trace": bool,
  "verification_agent_ids": ["uuid"]
}
```

### EdgeRequest
```json
{ "from_step_id": "uuid", "to_step_id": "uuid" }
```

---

## 24. System Config

**Module:** `src/server/api/system_config/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/system-config` | 200 | List configs (`?config_type=` filter) |
| POST | `/api/system-config` | 200 | Upsert config |
| DELETE | `/api/system-config/{key}` | 204 | Delete config |

### CreateSystemConfigRequest
```json
{
  "config_type": "string (required)",
  "config_key": "string (required)",
  "config_value": { /* JSON */ },
  "description": "string"
}
```

---

## 25. Health

**Module:** `src/server/api/health/mod.rs`

| Method | Path | Status | Description |
|--------|------|--------|-------------|
| GET | `/api/health` | 200 | Health check |

### HealthResponse
```json
{ "status": "ok", "version": "string", "db_connected": bool }
```

---

## Cross-Cutting Patterns

### Error Handling
All handlers return `Result<Json<T>, AppError>`. `AppError` variants:
- `BadRequest(String)` -> 400
- `Unauthorized(String)` -> 401
- `Forbidden(String)` -> 403
- `NotFound(String)` -> 404
- `Conflict(String)` -> 409
- `ServiceUnavailable(String)` -> 503
- `Internal(String)` -> 500

Error response format: `{ "error": "message", "status": 400 }`

### Ownership
- Ownership verification returns 404 (not 403) to avoid information leakage
- Validated on all resource CRUD operations via `auth.user_id`

### Async Operations (202 Accepted)
- Chat messages, collection runs, room messages, interactive execution messages
- Response includes an ID for polling/streaming

### Streaming
- SSE for real-time responses (chat, executions)
- Buffered chunks replayed on late connects
- Event types: `token`, `tool_start`, `tool_end`, `doc_update`, `done`, `error`

### Pagination
- `?limit=N&offset=M` for list endpoints
- `?status=pending` for filtering
- `?q=search_term` for full-text search

### Repository Access
- `state.repo()` - Main ServerRepo
- `state.repos()` - Typed repository bundle (14 repositories)
