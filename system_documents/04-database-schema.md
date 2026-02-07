# Nexor Database Schema & Data Models

Complete reference for all database tables, relationships, Rust row types, and enums.

---

## 1. Migrations

All in `/migrations/` directory. Initial schema consolidated from 70+ incremental migrations.

| Migration | Description |
|-----------|-------------|
| 0001 | Consolidated initial schema (all tables) |
| 0002 | Remove user_id from tools (system-wide) |
| 0003 | agent_guidances table (persistent feedback) |
| 0004 | workflow container config (container_enabled, target_repo_url, target_branch) |
| 0005 | reasoning_trace flag on workflow_steps |
| 0006 | is_exemplary flag on agent_executions (few-shot learning) |
| 0007 | vpn_enabled flag on workflows |
| 0008 | verification_agent_ids JSONB on workflow_steps |
| 0009 | is_admin flag on users (security hardening) |

---

## 2. Entity Relationship Map

```
users (root entity)
 |-- agents
 |    |-- agent_guidances (learned feedback)
 |    |-- agent_tools (M2M -> tools)
 |    |-- agent_context (M2M -> documents)
 |    +-- agent_executions
 |         |-- execution_messages (LLM turns)
 |         +-- results (saved outputs)
 |
 |-- workflows
 |    |-- workflow_steps (DAG nodes)
 |    |    |-- step_inputs (input port defs)
 |    |    |-- step_outputs (output port defs)
 |    |    |-- step_routing_rules (label routing)
 |    |    |-- step_documents (M2M -> documents)
 |    |    +-- workflow_step_agents (multi-agent config)
 |    +-- workflow_step_edges (DAG edges)
 |
 |-- workflow_collections
 |    |-- collection_workflows (M2M -> workflows)
 |    |-- collection_workflow_edges (DAG edges)
 |    +-- collection_runs
 |         +-- workflow_executions
 |
 |-- rooms
 |    |-- room_members (M2M -> agents)
 |    +-- room_sessions
 |         +-- room_execution_outputs
 |
 |-- documents
 |-- output_schemas
 |-- prompt_templates
 |-- tool_routers
 |    |-- tool_router_modes
 |    |    |-- tool_router_mode_tools (M2M -> tools)
 |    |    +-- mode_required_capabilities (M2M -> tool_capabilities)
 |    +-- tool_router_tools (M2M -> tools)
 |
 |-- chat_sessions
 |    |-- chat_messages
 |    +-- context_store
 |
 |-- tasks
 |-- token_ledger
 +-- router_requests

tools (system-wide, not user-scoped)
tool_capabilities (system-wide)
 +-- tool_capability_assignments (M2M -> tools)

system_config (admin-level)
auth_config (singleton)
```

---

## 3. Core Tables

### users
```sql
id              UUID PK (gen_random_uuid)
email           TEXT UNIQUE
password_hash   TEXT (nullable, optional if GitHub auth)
github_id       BIGINT UNIQUE (nullable)
github_login    TEXT (nullable)
github_token_encrypted TEXT (nullable)
is_admin        BOOL DEFAULT false
created_at      TIMESTAMPTZ DEFAULT now()
updated_at      TIMESTAMPTZ DEFAULT now()
```

### auth_config (singleton)
```sql
id              INT PK (CHECK id = 1)
password_hash   TEXT
created_at      TIMESTAMPTZ DEFAULT now()
```

---

### agents
```sql
id                UUID PK (gen_random_uuid)
user_id           UUID FK -> users
name              TEXT
system_prompt     TEXT DEFAULT ''
persona_style     TEXT DEFAULT 'casual'
model_provider    TEXT DEFAULT 'anthropic'
model_id          TEXT
model_max_tokens  INT DEFAULT 4096
model_temperature REAL DEFAULT 0.7
router_mode       BOOL DEFAULT false
router_id         UUID FK -> tool_routers (nullable)
output_schema_id  UUID FK -> output_schemas (nullable)
status            TEXT DEFAULT 'idle'
current_task      UUID (nullable)
version           INT DEFAULT 1
created_at        TIMESTAMPTZ DEFAULT now()
```
**Status values:** idle, working, waiting_for_context, waiting_for_approval
**Versioning:** agents_versions table tracks all changes

### agent_guidances
```sql
id                UUID PK (gen_random_uuid)
agent_id          UUID FK -> agents ON DELETE CASCADE
workflow_step_id  UUID FK -> workflow_steps ON DELETE SET NULL (nullable)
suggestions       JSONB DEFAULT '[]'  -- array of strings
source            TEXT DEFAULT 'manual'
version           INT DEFAULT 1
is_active         BOOL DEFAULT true
created_at        TIMESTAMPTZ DEFAULT now()
updated_at        TIMESTAMPTZ DEFAULT now()
```

### agent_tools (M2M)
```sql
agent_id  UUID FK -> agents
tool_id   UUID FK -> tools
PK: (agent_id, tool_id)
```

### agent_context (M2M)
```sql
agent_id     UUID FK -> agents
document_id  UUID FK -> documents
PK: (agent_id, document_id)
```

---

### tools (system-wide)
```sql
id            UUID PK (gen_random_uuid)
name          TEXT UNIQUE
display_name  TEXT
description   TEXT
parameters    JSONB DEFAULT '{}'  -- JSON Schema
created_at    TIMESTAMPTZ DEFAULT now()
version       INT DEFAULT 1
```

### tool_routers
```sql
id               UUID PK (gen_random_uuid)
user_id          UUID FK -> users
name             TEXT
description      TEXT (nullable)
system_prompt    TEXT
model_id         TEXT
is_active        BOOL DEFAULT true
parent_router_id UUID FK -> tool_routers (nullable)
level            INT DEFAULT 1 (CHECK: 1, 2, or 3)
created_at       TIMESTAMPTZ DEFAULT now()
updated_at       TIMESTAMPTZ DEFAULT now()
```

### tool_router_modes
```sql
id                            UUID PK (gen_random_uuid)
router_id                     UUID FK -> tool_routers
mode_key                      TEXT (regex: ^[a-z][a-z0-9_]*$)
display_name                  TEXT
description                   TEXT
system_prompt                 TEXT
temperature                   REAL DEFAULT 0.7 (CHECK: 0.0-2.0)
max_tokens                    INT DEFAULT 4096 (CHECK: > 0)
append_to_agent_system_prompt BOOL DEFAULT false
append_to_agent_tools         BOOL DEFAULT true
display_order                 INT DEFAULT 0
created_at                    TIMESTAMPTZ DEFAULT now()
updated_at                    TIMESTAMPTZ DEFAULT now()
UNIQUE: (router_id, mode_key)
```

### tool_capabilities
```sql
id              UUID PK (gen_random_uuid)
capability_key  TEXT UNIQUE (regex: ^[a-z][a-z0-9_]*$)
display_name    TEXT
category        TEXT  -- code_analysis, file_operations, testing, etc.
safety_level    TEXT DEFAULT 'safe'  -- safe, caution, dangerous
description     TEXT
created_at      TIMESTAMPTZ DEFAULT now()
```

### tool_capability_assignments (M2M)
```sql
tool_id        UUID FK -> tools
capability_id  UUID FK -> tool_capabilities
PK: (tool_id, capability_id)
```

### mode_required_capabilities (M2M)
```sql
mode_id        UUID FK -> tool_router_modes
capability_id  UUID FK -> tool_capabilities
is_required    BOOL DEFAULT true
PK: (mode_id, capability_id)
```

---

### documents
```sql
id          UUID PK
user_id     UUID FK -> users
session_id  UUID FK -> chat_sessions (nullable)
title       TEXT
content     TEXT DEFAULT ''
summary     TEXT (nullable)
doc_type    TEXT DEFAULT 'architecture'
ref_tag     TEXT DEFAULT ''
tags        TEXT[] DEFAULT '{}'
created_at  TIMESTAMPTZ DEFAULT now()
updated_at  TIMESTAMPTZ DEFAULT now()
```

### output_schemas
```sql
id         UUID PK (gen_random_uuid)
user_id    UUID FK -> users
name       TEXT
schema     JSONB  -- JSON Schema
created_at TIMESTAMPTZ DEFAULT now()
version    INT DEFAULT 1
UNIQUE: (user_id, name)
```

### prompt_templates
```sql
id         UUID PK (gen_random_uuid)
user_id    UUID FK -> users
name       TEXT
content    TEXT  -- Template with {variable} placeholders
created_at TIMESTAMPTZ DEFAULT now()
version    INT DEFAULT 1
UNIQUE: (user_id, name)
```

---

## 4. Workflow Tables

### workflows
```sql
id                UUID PK (gen_random_uuid)
user_id           UUID FK -> users
name              TEXT
description       TEXT DEFAULT ''
execution_mode    TEXT DEFAULT 'parallel'
version           INT DEFAULT 1
created_at        TIMESTAMPTZ DEFAULT now()
container_enabled BOOL DEFAULT false
target_repo_url   TEXT (nullable)
target_branch     TEXT (nullable)
vpn_enabled       BOOL DEFAULT false
```

### workflow_steps (DAG nodes)
```sql
id                       UUID PK (gen_random_uuid)
workflow_id              UUID FK -> workflows
agent_id                 UUID FK -> agents
execution_mode           TEXT DEFAULT 'single'
agent_execution_mode     TEXT (nullable)  -- sequential/parallel
for_each_ref             TEXT (nullable)  -- JSONPath to array
for_each_label_field     TEXT (nullable)
routing_mode             TEXT (nullable)  -- NULL, "label", "cavernous"
routing_field            TEXT (nullable)
cavernous_config_document_id UUID FK -> documents (nullable)
prompt_template_id       UUID FK -> prompt_templates (nullable)
prompt_template          TEXT DEFAULT ''
output_schema_id         UUID FK -> output_schemas (nullable)
output_variable_name     TEXT (nullable)  -- DEPRECATED: use step_outputs
room_id                  UUID FK -> rooms (nullable)
interactive_agent_id     UUID FK -> agents (nullable)
verification_agent_ids   JSONB (nullable)  -- Array of UUIDs
reasoning_trace          BOOL DEFAULT false
display_order            INT DEFAULT 0
version                  INT DEFAULT 1
position_x               DOUBLE (nullable)  -- Canvas position
position_y               DOUBLE (nullable)
width                    DOUBLE DEFAULT 200
height                   DOUBLE DEFAULT 100
```

**execution_mode values:**
| Mode | Tier | Description |
|------|------|-------------|
| `single` | 1 | Execute once with assigned agent |
| `for_each` | 2 | Iterate array, optional label-based routing |
| `cavernous` | 3 | Document-based dynamic routing |
| `room` | 4 | Multi-agent room discussion |

### workflow_step_edges (DAG edges)
```sql
id                 UUID PK (gen_random_uuid)
workflow_id        UUID FK -> workflows
from_step_id       UUID FK -> workflow_steps
to_step_id         UUID FK -> workflow_steps
from_output_port   TEXT (nullable)
to_input_port      TEXT (nullable)
transform_jsonpath TEXT (nullable)
condition_type     TEXT (nullable)
condition_value    JSONB (nullable)
edge_label         TEXT (nullable)  -- For-each routing label
UNIQUE: (workflow_id, from_step_id, to_step_id)
```

### step_inputs
```sql
id                UUID PK (gen_random_uuid)
workflow_step_id  UUID FK -> workflow_steps
port_name         TEXT
port_type         TEXT  -- text, json, array, object, etc.
required          BOOL DEFAULT false
default_value     JSONB (nullable)
description       TEXT (nullable)
json_schema       JSONB (nullable)
created_at        TIMESTAMPTZ DEFAULT now()
UNIQUE: (workflow_step_id, port_name)
```

### step_outputs
```sql
id                UUID PK (gen_random_uuid)
workflow_step_id  UUID FK -> workflow_steps
port_name         TEXT
port_type         TEXT
json_path         TEXT  -- JSONPath to extract from envelope.data
description       TEXT (nullable)
json_schema       JSONB (nullable)
created_at        TIMESTAMPTZ DEFAULT now()
UNIQUE: (workflow_step_id, port_name)
```

### step_routing_rules
```sql
id                UUID PK (gen_random_uuid)
workflow_step_id  UUID FK -> workflow_steps
label_value       TEXT  -- Category/label value
agent_id          UUID FK -> agents  -- Specialist for this label
description       TEXT (nullable)
display_order     INT DEFAULT 0
created_at        TIMESTAMPTZ DEFAULT now()
UNIQUE: (workflow_step_id, label_value)
```

### step_documents (M2M)
```sql
step_id      UUID FK -> workflow_steps
document_id  UUID FK -> documents
PK: (step_id, document_id)
```

---

## 5. Collection Tables

### workflow_collections
```sql
id              UUID PK (gen_random_uuid)
user_id         UUID FK -> users
name            TEXT
description     TEXT (nullable)
execution_mode  TEXT DEFAULT 'parallel'
created_at      TIMESTAMPTZ DEFAULT now()
updated_at      TIMESTAMPTZ DEFAULT now()
```

### collection_workflows (M2M)
```sql
collection_id   UUID FK -> workflow_collections
workflow_id     UUID FK -> workflows
display_order   INT DEFAULT 0
execution_mode  TEXT (nullable)  -- Override parent
PK: (collection_id, workflow_id)
```

### collection_workflow_edges
```sql
from_workflow_id UUID FK -> workflows
to_workflow_id   UUID FK -> workflows
collection_id    UUID FK -> workflow_collections
PK: (from_workflow_id, to_workflow_id, collection_id)
```

### collection_runs
```sql
id              UUID PK (gen_random_uuid)
collection_id   UUID FK -> workflow_collections
user_id         UUID FK -> users
status          TEXT  -- pending, running, success, error
started_at      TIMESTAMPTZ DEFAULT now()
completed_at    TIMESTAMPTZ (nullable)
error           TEXT (nullable)
```

### workflow_executions
```sql
id                UUID PK (gen_random_uuid)
collection_run_id UUID FK -> collection_runs
workflow_id       UUID FK -> workflows
user_id           UUID FK -> users
status            TEXT
started_at        TIMESTAMPTZ (nullable)
completed_at      TIMESTAMPTZ (nullable)
outputs           JSONB (nullable)  -- Aggregated step outputs
error             TEXT (nullable)
```

---

## 6. Execution Tables

### agent_executions
```sql
id                          UUID PK (gen_random_uuid)
agent_id                    UUID FK -> agents
workflow_step_id            UUID FK -> workflow_steps (nullable)
workflow_execution_id       UUID FK -> workflow_executions (nullable)
is_interactive              BOOL DEFAULT false
parent_agent_execution_id   UUID FK -> agent_executions (nullable)
system_prompt_rendered      TEXT
input                       TEXT
output                      TEXT (nullable)
structured_output           JSONB (nullable)
selected_mode_id            UUID (nullable)
selected_router_mode_id     UUID FK -> tool_router_modes (nullable)
room_session_id             UUID FK -> room_sessions (nullable)
speaker_order               INT (nullable)
status                      TEXT DEFAULT 'running'
started_at                  TIMESTAMPTZ DEFAULT now()
completed_at                TIMESTAMPTZ (nullable)
routing_analysis            JSONB (nullable)  -- Cavernous search results
selected_routing_document_id UUID FK -> documents (nullable)
is_exemplary                BOOL DEFAULT false
```

**Key indexes:**
- `(agent_id, workflow_step_id) WHERE is_exemplary = true` (few-shot lookup)
- `routing_analysis` (GIN, JSONB search)
- `started_at DESC` (recency)

### execution_messages
```sql
id                   UUID PK (gen_random_uuid)
agent_execution_id   UUID FK -> agent_executions
role                 TEXT  -- user, assistant, tool
content              TEXT
tool_call_id         TEXT (nullable)
input_tokens         BIGINT DEFAULT 0
output_tokens        BIGINT DEFAULT 0
created_at           TIMESTAMPTZ DEFAULT now()
```

### results
```sql
id                  UUID PK (gen_random_uuid)
user_id             UUID FK -> users
agent_execution_id  UUID FK -> agent_executions
output_schema_id    UUID FK -> output_schemas (nullable)
name                TEXT
data                JSONB
created_at          TIMESTAMPTZ DEFAULT now()
```

### token_ledger
```sql
id                  UUID PK (gen_random_uuid)
user_id             UUID FK -> users
agent_execution_id  UUID FK -> agent_executions (nullable)
model_id            TEXT
input_tokens        BIGINT
output_tokens       BIGINT
cost_usd            REAL
created_at          TIMESTAMPTZ DEFAULT now()
```

---

## 7. Room Tables

### rooms
```sql
id                       UUID PK (gen_random_uuid)
user_id                  UUID FK -> users
collection_id            UUID FK -> workflow_collections (nullable)
name                     TEXT
gatekeeper_enabled       BOOL DEFAULT false
gatekeeper_model_id      TEXT DEFAULT 'claude-haiku-4-20250414'
max_speakers_per_turn    INT DEFAULT 4
max_turns                INT DEFAULT 20
tools_enabled            BOOL DEFAULT false
default_output_schema_id UUID FK -> output_schemas (nullable)
aggregation_mode         TEXT DEFAULT 'final_speaker'
created_at               TIMESTAMPTZ DEFAULT now()
updated_at               TIMESTAMPTZ DEFAULT now()
```

**aggregation_mode values:** `final_speaker`, `consensus`, `all_outputs`

### room_members
```sql
room_id           UUID FK -> rooms
agent_id          UUID FK -> agents
display_name      TEXT (nullable)
role_description  TEXT
display_order     INT DEFAULT 0
input_schema_id   UUID FK -> output_schemas (nullable)
output_schema_id  UUID FK -> output_schemas (nullable)
output_name       TEXT (nullable)
PK: (room_id, agent_id)
```

### room_sessions
```sql
id                  UUID PK (gen_random_uuid)
room_id             UUID FK -> rooms
run_id              UUID (nullable)
status              TEXT DEFAULT 'active'
current_turn        INT DEFAULT 0
transcript_summary  TEXT (nullable)
started_at          TIMESTAMPTZ DEFAULT now()
completed_at        TIMESTAMPTZ (nullable)
structured_outputs  JSONB (nullable)
final_decision      JSONB (nullable)
```

### room_execution_outputs
```sql
id                  UUID PK (gen_random_uuid)
room_session_id     UUID FK -> room_sessions
agent_execution_id  UUID FK -> agent_executions
agent_id            UUID FK -> agents
speaker_order       INT
turn_number         INT
output_name         TEXT
structured_output   JSONB
raw_output          TEXT
schema_id           UUID FK -> output_schemas (nullable)
created_at          TIMESTAMPTZ DEFAULT now()
UNIQUE: (room_session_id, turn_number, output_name)
```

---

## 8. Chat & Context Tables

### chat_sessions
```sql
id            UUID PK
user_id       UUID FK -> users
mode_id       TEXT
title         TEXT DEFAULT ''
summary       TEXT DEFAULT ''
agent_id      UUID FK -> agents (nullable)
draft_config  JSONB (nullable)
created_at    TIMESTAMPTZ DEFAULT now()
updated_at    TIMESTAMPTZ DEFAULT now()
```

### chat_messages
```sql
id         UUID PK
user_id    UUID FK -> users
session_id UUID FK -> chat_sessions (nullable)
role       TEXT CHECK ('user', 'assistant')
content    TEXT
timestamp  TIMESTAMPTZ DEFAULT now()
```

### context_store
```sql
id          UUID PK (gen_random_uuid)
session_id  UUID FK -> chat_sessions
source      TEXT  -- documents, memory, output, etc.
priority    REAL DEFAULT 0.5
content     TEXT
metadata    JSONB (nullable)
status      TEXT DEFAULT 'active'
created_at  TIMESTAMPTZ DEFAULT now()
expires_at  TIMESTAMPTZ (nullable)
```

### router_requests
```sql
id                  UUID PK (gen_random_uuid)
session_id          UUID FK -> chat_sessions
agent_execution_id  UUID FK -> agent_executions (nullable)
intent              TEXT
priority            TEXT DEFAULT 'normal'
callback_hint       TEXT (nullable)
routed_tool         TEXT (nullable)
routed_args         JSONB (nullable)
is_async            BOOL DEFAULT false
passdown            TEXT (nullable)
chain               JSONB (nullable)
status              TEXT DEFAULT 'pending'
result              TEXT (nullable)
created_at          TIMESTAMPTZ DEFAULT now()
completed_at        TIMESTAMPTZ (nullable)
```

---

## 9. Supporting Tables

### system_config
```sql
id            UUID PK (gen_random_uuid)
config_type   TEXT
config_key    TEXT UNIQUE
config_value  JSONB
description   TEXT (nullable)
created_by    UUID FK -> users (nullable)
created_at    TIMESTAMPTZ DEFAULT now()
updated_at    TIMESTAMPTZ DEFAULT now()
```

### tasks
```sql
id              UUID PK
user_id         UUID FK -> users
slice_id        UUID FK -> vertical_slices (nullable)
title           TEXT
description     TEXT DEFAULT ''
assigned_agent  UUID FK -> agents (nullable)
status          TEXT DEFAULT 'pending'
priority        TEXT DEFAULT 'normal'
context_files   JSONB DEFAULT '[]'
metadata        JSONB (nullable)
retry_count     INT DEFAULT 0
max_retries     INT DEFAULT 3
last_error      TEXT (nullable)
created_at      TIMESTAMPTZ DEFAULT now()
updated_at      TIMESTAMPTZ DEFAULT now()
```

### pr_merge_queue
```sql
id             UUID PK
user_id        UUID FK -> users
repo_owner     TEXT
repo_name      TEXT
pr_number      INT
queue_position INT
status         TEXT DEFAULT 'pending'
conflict_info  TEXT (nullable)
error_message  TEXT (nullable)
created_at     TIMESTAMPTZ
updated_at     TIMESTAMPTZ
UNIQUE: (repo_owner, repo_name, pr_number)
```

---

## 10. Execution Envelope Format

All step executions return a standardized envelope:

### StepExecutionEnvelope
```json
{
  "status": "success|error|partial",
  "data": { /* actual output */ },
  "metadata": {
    "execution_id": "uuid",
    "execution_time_ms": 1500,
    "tokens_in": 2048,
    "tokens_out": 512,
    "cost_usd": 0.015,
    "model": "claude-opus-4",
    "agent_id": "uuid",
    "routing_label": "requirement_analysis",
    "selected_routing_document_id": "uuid"
  },
  "error": null
}
```

### ForEachAggregateEnvelope
```json
{
  "status": "partial",
  "data": [ /* array of StepExecutionEnvelope */ ],
  "metadata": {
    "total_iterations": 10,
    "successful_iterations": 8,
    "failed_iterations": 2,
    "execution_time_ms": 5000,
    "total_tokens_in": 20000,
    "total_tokens_out": 5000,
    "total_cost_usd": 0.15,
    "routing_distribution": { "label1": 5, "label2": 3, "label3": 2 }
  },
  "errors": [{ "iteration_index": 3, "iteration_label": "item3", "message": "..." }]
}
```

---

## 11. Key Rust Enums

### Agent/Task Status
```rust
enum AgentStatus { Idle, Working, WaitingForContext, WaitingForApproval }
enum TaskStatus { Pending, InProgress, Review, Completed, Failed }
enum Priority { Low, Normal, High, Urgent }
```

### Execution
```rust
enum ExecutionStatus { Success, Error, Partial }
```

### Configuration
```rust
enum AutonomyLevel { FullAuto, ApprovalGates, Supervised }
enum GitStrategy { BranchPerSlice, BranchPerTicket }
enum SandboxMode { Docker, LocalRestricted, None }
```

---

## 12. Versioning Pattern

Major entities track change history:

```
Entity (current version)
 +-- id, user_id, fields..., version INT

Entity_versions (snapshots)
 +-- id, version (composite PK)
 +-- all fields from Entity
 +-- changed_by UUID
 +-- changed_at TIMESTAMPTZ
```

Versioned entities: agents, agent_modes, tools, workflows, workflow_steps, output_schemas, prompt_templates.

---

## 13. Rust Row Types

All derive `Clone, Debug, Serialize, sqlx::FromRow`. Defined in `src/db/mod.rs`.

| Row Type | Table |
|----------|-------|
| `AgentRow` | agents |
| `AgentGuidanceRow` | agent_guidances |
| `ToolRow` | tools |
| `AgentExecutionRow` | agent_executions |
| `ExecutionMessageRow` | execution_messages |
| `WorkflowRow` | workflows |
| `WorkflowStepRow` | workflow_steps |
| `WorkflowStepEdgeRow` | workflow_step_edges |
| `StepInputRow` | step_inputs |
| `StepOutputRow` | step_outputs |
| `StepRoutingRuleRow` | step_routing_rules |
| `WorkflowCollectionRow` | workflow_collections |
| `CollectionWorkflowRow` | collection_workflows |
| `CollectionRunRow` | collection_runs |
| `WorkflowExecutionRow` | workflow_executions |
| `RoomRow` | rooms |
| `RoomMemberRow` | room_members |
| `RoomSessionRow` | room_sessions |
| `RoomExecutionOutputRow` | room_execution_outputs |
| `DocumentRow` | documents |
| `OutputSchemaRow` | output_schemas |
| `PromptTemplateRow` | prompt_templates |
| `ResultRow` | results |
| `TokenLedgerRow` | token_ledger |
| `ToolRouterRow` | tool_routers |
| `ToolRouterModeRow` | tool_router_modes |
| `ToolCapabilityRow` | tool_capabilities |
| `ContextStoreRow` | context_store |
| `RouterRequestRow` | router_requests |
| `SystemConfigRow` | system_config |
| `UserRow` | users |

---

## 14. Frontend Data Model Notes

Key patterns to understand:

1. **Execution Envelopes** - All step results wrapped in standardized structures with `status`, `data`, `metadata`, `error`
2. **Ports & Data Flow** - Steps connect via named input/output ports. Output ports use `json_path` to extract from envelopes.
3. **Four Execution Tiers:**
   - Tier 1: Static (`execution_mode="single"`)
   - Tier 2: Label routing (`execution_mode="for_each"` + `routing_mode="label"`)
   - Tier 3: Document routing (`execution_mode="cavernous"`)
   - Tier 4: Room (`execution_mode="room"`)
4. **Token Tracking** - Every LLM call audited in token_ledger with cost_usd
5. **Capability Taxonomy** - Tools advertise capabilities; modes require capabilities; auto-matching
6. **Versioning** - Major entities keep full history for audit
