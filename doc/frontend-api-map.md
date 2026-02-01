# Frontend API Map

Reference diagram for building the React frontend. Maps every API endpoint, domain model, and WS channel the frontend consumes.

## REST Endpoints by Domain

### Auth
| Method | Path | Returns |
|--------|------|---------|
| POST | /api/auth/register | `{token, expires_in, user}` |
| POST | /api/auth/login | `{token, expires_in}` |
| GET | /api/auth/me | `{user, authenticated, token_expires}` |

### Sessions & Chat
| Method | Path | Returns |
|--------|------|---------|
| GET | /api/modes | `[{id, name, description}]` |
| GET | /api/sessions | `[SessionResponse]` |
| POST | /api/sessions | `SessionResponse` |
| GET | /api/sessions/:id | `SessionResponse` |
| PATCH | /api/sessions/:id | `SessionResponse` |
| DELETE | /api/sessions/:id | 204 |
| POST | /api/sessions/:id/chat | `{message_id, status}` |
| GET | /api/sessions/:id/history | `[{id, role, content, timestamp}]` |
| GET | /api/sessions/:id/chat/:mid/stream | SSE: token, tool_start, tool_end, doc_update, done, error |

### Agents
| Method | Path | Returns |
|--------|------|---------|
| GET | /api/agents | `{stats: {orchestrators, workers, utilities}, agents: [AgentResponse]}` |
| POST | /api/agents | `AgentResponse` |
| GET | /api/agents/:id | `AgentResponse` |
| PATCH | /api/agents/:id | `AgentResponse` |
| DELETE | /api/agents/:id | 204 |
| GET | /api/agents/:id/tools | `AgentToolsResponse` |
| PUT | /api/agents/:id/tools | `AgentToolsResponse` |
| GET | /api/agents/:id/context | `{agent_id, documents: [DocumentListItem]}` |
| PUT | /api/agents/:id/context | `AgentContextResponse` |

### Tasks
| Method | Path | Returns |
|--------|------|---------|
| GET | /api/tasks | `[Task]` (query: status, limit) |
| POST | /api/tasks | `Task` |
| GET | /api/tasks/:id | `Task` |

### Pipelines
| Method | Path | Returns |
|--------|------|---------|
| POST | /api/pipelines/:id/stages/:n/render | `{pipeline_id, stage_number, stage_name, prompt}` |
| GET | /api/pipelines/:id/stages/:n/side-tasks | `[SideTaskResponse]` |
| POST | /api/pipelines/:id/stages/:n/side-tasks | `SideTaskResponse` |
| DELETE | /api/pipelines/:id/stages/:n/side-tasks/:sid | 204 |

### Pipeline Runs
| Method | Path | Returns |
|--------|------|---------|
| GET | /api/pipeline-runs | `[PipelineRunResponse]` (query: pipeline_id) |
| GET | /api/pipeline-runs/:id | `PipelineRunDetailResponse` (includes stage executions) |
| POST | /api/pipeline-runs/:id/approve | `{...}` (body: user_input?) |

### Tools
| Method | Path | Returns |
|--------|------|---------|
| GET | /api/tools | `[ToolResponse]` |
| POST | /api/tools | `ToolResponse` |
| GET | /api/tools/:id | `ToolResponse` |
| PATCH | /api/tools/:id | `ToolResponse` |
| DELETE | /api/tools/:id | 204 |

### Documents
| Method | Path | Returns |
|--------|------|---------|
| GET | /api/documents | `[DocumentListItem]` |
| POST | /api/documents | `DocumentResponse` |
| GET | /api/documents/search | `[DocumentSearchResult]` (query: q) |
| GET | /api/documents/:id | `DocumentResponse` |
| PATCH | /api/documents/:id | `DocumentResponse` |
| DELETE | /api/documents/:id | 204 |

### Config & Stats
| Method | Path | Returns |
|--------|------|---------|
| GET | /api/config | `ConfigResponse` |
| PATCH | /api/config | `ConfigResponse` |
| GET | /api/stats | `[UsageSummaryRow]` (last 24h) |
| GET | /api/health | `{status, version, db_connected}` |

---

## Domain Models (Frontend Shapes)

### User
```
id: string, email: string, github_login: string | null
```

### Agent
```
id: string, tier: "orchestrator" | "worker" | "utility",
persona: {name, system_prompt, style}, model_config: {provider, model_id, max_tokens, temperature},
status: "idle" | "working" | "waiting_for_context" | "waiting_for_approval",
current_task: string | null, router_mode: boolean
```

### Task
```
id: string, slice_id: string | null, title: string, description: string,
assigned_tier: string, assigned_agent: string | null,
status: "pending" | "in_progress" | "review" | "completed" | "failed",
priority: "low" | "normal" | "high" | "urgent",
context_files: string[], metadata: Record<string, string> | null,
depends_on: string[], retry_count: number, max_retries: number, last_error: string | null,
created_at: string, updated_at: string
```

### Session
```
id: string, user_id: string, mode_id: string,
title: string, summary: string, created_at: string, updated_at: string
```

### ChatMessage
```
id: string, role: "user" | "assistant", content: string, timestamp: string
```

### Document
```
id: string, user_id: string, session_id: string | null,
title: string, content: string, summary: string,
doc_type: string, ref_tag: string, tags: string[],
created_at: string, updated_at: string
```

### DocumentSearchResult
```
id: string, title: string, summary: string, ref_tag: string, snippet: string
```

### Pipeline (inferred from rows)
```
id: string, name: string,
stages: [{stage_number, agent_id?, cluster_id?, role?, approval_required, fan_out,
          stage_name, input_definitions, output_description, output_schema}]
```

### PipelineRun
```
id: string, pipeline_id: string, user_id: string,
status: string, initial_task: string, stage_outputs: Record<string, unknown>,
current_stage: number, started_at: string, completed_at: string | null,
total_input_tokens: number, total_output_tokens: number
```

### StageExecution
```
id: string, run_id: string, stage_number: number, stage_name: string,
agent_id: string | null, status: string,
rendered_prompt: string | null, output: string | null, structured_output: unknown | null,
user_input: string | null, input_tokens: number, output_tokens: number,
started_at: string, completed_at: string | null, duration_ms: number
```

### Tool
```
id: string, name: string, description: string, category: string,
parameter_schema: unknown, output_schema: unknown,
enabled: boolean, cluster_id: string | null, is_builtin: boolean
```

### FeedItem
```
id: string, agent_id: string, content: string,
item_type: "agent_report" | "task_started" | "task_completed" | "error" | "user_message" | "system_notice" | "milestone",
verbosity_level: "quiet" | "normal" | "verbose", timestamp: string
```

### UsageSummary
```
tier: string, model_id: string, total_input: number, total_output: number, call_count: number
```

### Config
```
verbosity: string, models: {orchestrator, worker, utility},
pool: {max_orchestrators, max_workers, max_utilities},
autonomy: string, git_strategy: string, sandbox_mode: string
```

---

## WebSocket Channels

Single connection at `ws://.../ws?token=<jwt>`. Client subscribes per channel.

| Channel | Payload | Used By |
|---------|---------|---------|
| feed | FeedItem (agent activity) | Dashboard, Feed panel |
| tasks | TaskUpdate {task_id, status, progress?} | Task board, Dashboard |
| agents | AgentUpdate {agent_id, status, current_task_id?} | Agent list, Dashboard |
| sessions | SessionUpdate {session_id, message, sender_role} | Chat interface |
| pipelines | PipelineUpdate {pipeline_id, run_id, current_stage, stage_status} | Pipeline runner |

### Protocol
```
Client → Server: {type: "subscribe", channels: ["feed", "tasks"]}
Client → Server: {type: "unsubscribe", channels: ["feed"]}
Server → Client: {channel: "tasks", task_id: "...", status: "...", ...}
```

---

## Context Architecture (1:1 with domains)

```
AuthProvider          ← /api/auth/*
  WebSocketProvider   ← /ws (single connection)
    AgentProvider     ← /api/agents + WS "agents"
    TaskProvider      ← /api/tasks + WS "tasks"
    PipelineProvider  ← /api/pipeline-runs + WS "pipelines"
    SessionProvider   ← /api/sessions + WS "sessions"
    FeedProvider      ← WS "feed" only
    StatsProvider     ← /api/stats (polling)
    DocumentProvider  ← /api/documents (on-demand)
```

Each domain context: REST for initial fetch, WS for real-time updates. Contexts subscribe to their WS channel on mount and unsubscribe on unmount via the WebSocketProvider's `subscribe()` API.
