# DAG Execution Schema — Records & Message Flow

What gets saved to the database when the DAG executor runs a pipeline step with tool use, and what it looks like when loaded in the frontend.

## Table Relationships

```
pipeline_runs
  └─ stage_executions
       └─ agent_executions
            ├─ execution_messages  (the conversation log)
            └─ token_ledger        (per-LLM-call cost tracking)
```

## Record Creation Sequence

### 1. Pipeline Run Starts

```
┌─ pipeline_runs ──────────────────────────────────────────────┐
│ id: run-7a3b                                                 │
│ pipeline_id: pipe-001                                        │
│ status: 'running'                                            │
│ initial_task: 'Analyze the auth module for security issues'  │
│ current_stage: 1                                             │
│ total_input_tokens: 0                                        │
│ total_output_tokens: 0                                       │
└──────────────────────────────────────────────────────────────┘
```

### 2. Stage Execution Created

```
┌─ stage_executions ───────────────────────────────────────────┐
│ id: se-4f1c                                                  │
│ run_id: run-7a3b                                             │
│ stage_number: 1                                              │
│ stage_name: 'Security Audit'                                 │
│ agent_id: agent-009                                          │
│ status: 'running'                                            │
│ rendered_prompt: 'Review src/auth/mod.rs for OWASP...'       │
└──────────────────────────────────────────────────────────────┘
```

### 3. Agent Execution Created

```
┌─ agent_executions ───────────────────────────────────────────┐
│ id: ae-82d1                                                  │
│ stage_execution_id: se-4f1c                                  │
│ agent_id: agent-009                                          │
│ workflow_step_id: step-003                                   │
│ is_interactive: false                                        │
│ system_prompt_rendered: 'You are a security auditor...'      │
│ input: 'Review src/auth/mod.rs for OWASP top 10...'         │
│ status: 'running'                                            │
│ output: NULL              <- filled on completion            │
│ structured_output: NULL   <- filled on completion            │
│ input_tokens: 0           <- accumulated total on completion │
│ output_tokens: 0                                             │
│ cost_usd: 0.0                                                │
└──────────────────────────────────────────────────────────────┘
```

### 4. Execution Messages (the conversation log)

Messages are inserted in order as the LLM loop runs:

```
execution_messages (ordered by created_at)

┌─ MSG 1 ──────────────────────────────────────────────────────┐
│ agent_execution_id: ae-82d1                                  │
│ role: 'system'                                               │
│ content: 'You are a security auditor. Analyze code for       │
│           OWASP top 10 vulnerabilities. Respond with JSON    │
│           matching this schema: { "findings": [...] }'       │
│ tool_call_id: NULL                                           │
│ input_tokens: 0    output_tokens: 0                          │
└──────────────────────────────────────────────────────────────┘

┌─ MSG 2 ──────────────────────────────────────────────────────┐
│ agent_execution_id: ae-82d1                                  │
│ role: 'user'                                                 │
│ content: 'Review src/auth/mod.rs for OWASP top 10            │
│           vulnerabilities. The module handles JWT             │
│           validation and session management.'                 │
│ tool_call_id: NULL                                           │
│ input_tokens: 0    output_tokens: 0                          │
└──────────────────────────────────────────────────────────────┘

── LLM ROUND 1 (stop_reason: ToolUse) ────────────────────────

┌─ MSG 3 ──────────────────────────────────────────────────────┐
│ role: 'assistant'                                            │
│ content: 'I need to read the auth module first to            │
│           analyze it properly.'                              │
│ tool_call_id: NULL                                           │
│ input_tokens: 450    output_tokens: 85                       │
└──────────────────────────────────────────────────────────────┘

┌─ MSG 4 ──────────────────────────────────────────────────────┐
│ role: 'assistant'                                            │
│ content: '{"tool":"read_file",                               │
│            "input":{"path":"src/auth/mod.rs"}}'              │
│ tool_call_id: 'toolu_abc123'                                 │
│ input_tokens: 0    output_tokens: 0                          │
└──────────────────────────────────────────────────────────────┘

┌─ MSG 5 ──────────────────────────────────────────────────────┐
│ role: 'tool'                                                 │
│ content: 'pub fn validate_jwt(token: &str) -> Result...      │
│           // WARNING: no expiry check                        │
│           let claims = decode(token, &key, &val)...'         │
│ tool_call_id: 'toolu_abc123'                                 │
│ input_tokens: 0    output_tokens: 0                          │
└──────────────────────────────────────────────────────────────┘

── LLM ROUND 2 (stop_reason: EndTurn) ────────────────────────

┌─ MSG 6 ──────────────────────────────────────────────────────┐
│ role: 'assistant'                                            │
│ content: '{"findings": [                                     │
│   {"severity": "high",                                       │
│    "vuln": "Missing JWT expiry validation",                  │
│    "line": 42,                                               │
│    "recommendation": "Add exp claim check"},                 │
│   {"severity": "medium",                                     │
│    "vuln": "No rate limiting on token refresh",              │
│    "line": 78,                                               │
│    "recommendation": "Add rate limiter middleware"}           │
│  ]}'                                                         │
│ tool_call_id: NULL                                           │
│ input_tokens: 1800    output_tokens: 320                     │
└──────────────────────────────────────────────────────────────┘
```

### 5. Token Ledger (one entry per LLM call)

```
token_ledger

┌─ LEDGER 1 (round 1) ────────────────────────────────────────┐
│ user_id: user-001                                            │
│ agent_execution_id: ae-82d1                                  │
│ model_id: 'claude-sonnet-4-20250514'                         │
│ input_tokens: 450                                            │
│ output_tokens: 85                                            │
│ cost_usd: 0.0022                                             │
└──────────────────────────────────────────────────────────────┘

┌─ LEDGER 2 (round 2) ────────────────────────────────────────┐
│ user_id: user-001                                            │
│ agent_execution_id: ae-82d1                                  │
│ model_id: 'claude-sonnet-4-20250514'                         │
│ input_tokens: 1800                                           │
│ output_tokens: 320                                           │
│ cost_usd: 0.0088                                             │
└──────────────────────────────────────────────────────────────┘
```

### 6. Agent Execution Completion

```sql
UPDATE agent_executions SET
    status = 'completed',
    output = '{"findings": [{"severity": "high", ...}]}',
    structured_output = {"findings": [...]},   -- parsed JSONB
    input_tokens = 2250,                       -- 450 + 1800
    output_tokens = 405,                       -- 85 + 320
    cost_usd = 0.011,                          -- 0.0022 + 0.0088
    completed_at = NOW()
WHERE id = 'ae-82d1'
```

## Frontend View

When `useInteractiveChat('ae-82d1')` calls `GET /agent-executions/ae-82d1/messages`, it gets MSG 1-6 in order. The `tool_call_id` field links tool calls to their results:

```
┌─────────────────────────────────────────────────┐
│ SYSTEM: You are a security auditor...           │
├─────────────────────────────────────────────────┤
│ USER: Review src/auth/mod.rs for OWASP...       │
├─────────────────────────────────────────────────┤
│ ASSISTANT: I need to read the auth module...    │
│   tool: read_file("src/auth/mod.rs")            │  <- MSG 4 (tool_call_id set)
│   result: pub fn validate_jwt(token: &str)...   │  <- MSG 5 (same tool_call_id)
├─────────────────────────────────────────────────┤
│ ASSISTANT: Found 2 issues:                      │
│   HIGH - Missing JWT expiry validation          │
│   MEDIUM - No rate limiting on refresh          │
└─────────────────────────────────────────────────┘
```

## Key Details

**Token accounting:**
- Each LLM call creates one `token_ledger` entry with that round's tokens
- Tool call/result messages get `input_tokens=0, output_tokens=0` (already counted in the assistant message)
- The final `agent_executions` row gets the accumulated totals

**Message roles:**
- `'system'` — initial system prompt (1 per execution)
- `'user'` — initial rendered prompt (1 per execution)
- `'assistant'` — LLM response text, or tool call JSON (multiple per execution)
- `'tool'` — tool execution result (1 per tool call)

**Tool call linking:**
- `tool_call_id` is NULL for plain text messages
- Tool call MSG and tool result MSG share the same `tool_call_id` (e.g., `'toolu_abc123'`)
- This is how the frontend pairs them for display

**For-each expansion:**
- Steps with `execution_mode='for_each'` create N separate `agent_executions` (one per array element)
- Each gets its own independent message history and token ledger entries

**Interactive review:**
- If `step.interactive_agent_id` is set, a second `agent_execution` is created with `is_interactive=true` and `parent_agent_execution_id` pointing to the main execution
- Status is set to `'awaiting_user'` until approved via `POST /agent-executions/:id/approve`
