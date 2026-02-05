# Nexor Database Entity Relationship Diagram

## Core Entities

### users
```
┌─────────────────┐
│ users           │
├─────────────────┤
│ id (PK)         │
│ username        │
│ email           │
│ password_hash   │
│ created_at      │
│ updated_at      │
│ api_key         │
│ github_token    │
└─────────────────┘
```

### agents
```
┌──────────────────────────┐
│ agents                   │
├──────────────────────────┤
│ id (PK)                  │
│ user_id (FK → users)     │
│ name                     │
│ description              │
│ system_prompt            │
│ model                    │
│ temperature              │
│ max_tokens               │
│ router_id (FK)           │
│ output_schema_id (FK)    │
│ current_task (FK)        │
│ created_at               │
│ updated_at               │
│ version                  │
└──────────────────────────┘
```

### tasks
```
┌─────────────────────────┐
│ tasks                   │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ title                   │
│ description             │
│ status                  │
│ priority                │
│ assigned_agent_id       │
│ created_at              │
│ updated_at              │
│ completed_at            │
└─────────────────────────┘
```

### tools
```
┌─────────────────────────┐
│ tools                   │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ name                    │
│ description             │
│ schema                  │
│ implementation          │
│ created_at              │
│ updated_at              │
│ version                 │
└─────────────────────────┘
```

### documents
```
┌─────────────────────────┐
│ documents               │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ session_id (FK)         │
│ title                   │
│ content                 │
│ metadata                │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

## Workflow System

### workflows
```
┌──────────────────────────┐
│ workflows                │
├──────────────────────────┤
│ id (PK)                  │
│ user_id (FK → users)     │
│ name                     │
│ description              │
│ created_at               │
│ updated_at               │
│ version                  │
└──────────────────────────┘
```

### workflow_steps
```
┌─────────────────────────────────┐
│ workflow_steps                  │
├─────────────────────────────────┤
│ id (PK)                         │
│ workflow_id (FK → workflows)    │
│ agent_id (FK → agents)          │
│ room_id (FK → rooms)            │
│ name                            │
│ description                     │
│ prompt_template_id (FK)         │
│ output_schema_id (FK)           │
│ interactive_agent_id (FK)       │
│ step_order                      │
│ created_at                      │
│ updated_at                      │
│ version                         │
└─────────────────────────────────┘
```

### workflow_step_edges
```
┌────────────────────────────────┐
│ workflow_step_edges            │
├────────────────────────────────┤
│ from_step_id (FK → steps)      │
│ to_step_id (FK → steps)        │
│ condition                      │
│ created_at                     │
└────────────────────────────────┘
```

### workflow_collections
```
┌─────────────────────────────────┐
│ workflow_collections            │
├─────────────────────────────────┤
│ id (PK)                         │
│ user_id (FK → users)            │
│ name                            │
│ description                     │
│ created_at                      │
│ updated_at                      │
└─────────────────────────────────┘
```

### collection_workflows
```
┌─────────────────────────────────────┐
│ collection_workflows                │
├─────────────────────────────────────┤
│ collection_id (FK → collections)    │
│ workflow_id (FK → workflows)        │
│ workflow_order                      │
│ created_at                          │
└─────────────────────────────────────┘
```

### collection_workflow_edges
```
┌─────────────────────────────────────┐
│ collection_workflow_edges           │
├─────────────────────────────────────┤
│ collection_id (FK → collections)    │
│ from_workflow_id (FK → workflows)   │
│ to_workflow_id (FK → workflows)     │
│ condition                           │
│ created_at                          │
└─────────────────────────────────────┘
```

## Execution & Runtime

### workflow_executions
```
┌─────────────────────────────────────┐
│ workflow_executions                 │
├─────────────────────────────────────┤
│ id (PK)                             │
│ workflow_id (FK → workflows)        │
│ collection_run_id (FK)              │
│ user_id (FK → users)                │
│ status                              │
│ input                               │
│ output                              │
│ started_at                          │
│ completed_at                        │
│ error                               │
└─────────────────────────────────────┘
```

### agent_executions
```
┌─────────────────────────────────────────┐
│ agent_executions                        │
├─────────────────────────────────────────┤
│ id (PK)                                 │
│ agent_id (FK → agents)                  │
│ workflow_step_id (FK → steps)           │
│ workflow_execution_id (FK)              │
│ room_session_id (FK)                    │
│ parent_agent_execution_id (FK → self)   │
│ selected_mode_id (FK → agent_modes)     │
│ selected_router_mode_id (FK)            │
│ is_interactive                          │
│ system_prompt_rendered                  │
│ input                                   │
│ output                                  │
│ structured_output                       │
│ status                                  │
│ started_at                              │
│ completed_at                            │
│ error                                   │
│ total_tokens                            │
│ cost                                    │
└─────────────────────────────────────────┘
```

### execution_messages
```
┌──────────────────────────────────────┐
│ execution_messages                   │
├──────────────────────────────────────┤
│ id (PK)                              │
│ agent_execution_id (FK)              │
│ role                                 │
│ content                              │
│ tool_calls                           │
│ tool_results                         │
│ created_at                           │
└──────────────────────────────────────┘
```

### execution_variables
```
┌──────────────────────────────────────┐
│ execution_variables                  │
├──────────────────────────────────────┤
│ id (PK)                              │
│ collection_run_id (FK)               │
│ workflow_execution_id (FK)           │
│ step_execution_id (FK)               │
│ key                                  │
│ value                                │
│ created_at                           │
└──────────────────────────────────────┘
```

### collection_runs
```
┌─────────────────────────────────────┐
│ collection_runs                     │
├─────────────────────────────────────┤
│ id (PK)                             │
│ collection_id (FK → collections)    │
│ user_id (FK → users)                │
│ status                              │
│ input                               │
│ output                              │
│ started_at                          │
│ completed_at                        │
│ error                               │
└─────────────────────────────────────┘
```

## Chat & Sessions

### chat_sessions
```
┌─────────────────────────┐
│ chat_sessions           │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ agent_id (FK → agents)  │
│ title                   │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

### chat_messages
```
┌──────────────────────────┐
│ chat_messages            │
├──────────────────────────┤
│ id (PK)                  │
│ user_id (FK → users)     │
│ role                     │
│ content                  │
│ created_at               │
└──────────────────────────┘
```

### rooms
```
┌─────────────────────────────────┐
│ rooms                           │
├─────────────────────────────────┤
│ id (PK)                         │
│ user_id (FK → users)            │
│ collection_id (FK)              │
│ name                            │
│ description                     │
│ created_at                      │
│ updated_at                      │
└─────────────────────────────────┘
```

### room_members
```
┌─────────────────────────┐
│ room_members            │
├─────────────────────────┤
│ room_id (FK → rooms)    │
│ agent_id (FK → agents)  │
│ role                    │
│ joined_at               │
└─────────────────────────┘
```

### room_sessions
```
┌─────────────────────────┐
│ room_sessions           │
├─────────────────────────┤
│ id (PK)                 │
│ room_id (FK → rooms)    │
│ status                  │
│ started_at              │
│ completed_at            │
└─────────────────────────┘
```

## Tool Router System

### tool_routers
```
┌────────────────────────────────┐
│ tool_routers                   │
├────────────────────────────────┤
│ id (PK)                        │
│ user_id (FK → users)           │
│ parent_router_id (FK → self)   │
│ name                           │
│ description                    │
│ created_at                     │
│ updated_at                     │
└────────────────────────────────┘
```

### tool_router_modes
```
┌─────────────────────────────┐
│ tool_router_modes           │
├─────────────────────────────┤
│ id (PK)                     │
│ router_id (FK → routers)    │
│ name                        │
│ description                 │
│ created_at                  │
└─────────────────────────────┘
```

### tool_router_mode_tools
```
┌─────────────────────────────┐
│ tool_router_mode_tools      │
├─────────────────────────────┤
│ mode_id (FK → modes)        │
│ tool_id (FK → tools)        │
│ created_at                  │
└─────────────────────────────┘
```

### tool_router_tools
```
┌─────────────────────────────┐
│ tool_router_tools           │
├─────────────────────────────┤
│ router_id (FK → routers)    │
│ tool_id (FK → tools)        │
│ created_at                  │
└─────────────────────────────┘
```

## Agent Relations

### agent_tools
```
┌─────────────────────────┐
│ agent_tools             │
├─────────────────────────┤
│ agent_id (FK → agents)  │
│ tool_id (FK → tools)    │
│ created_at              │
└─────────────────────────┘
```

### agent_modes
```
┌─────────────────────────┐
│ agent_modes             │
├─────────────────────────┤
│ id (PK)                 │
│ agent_id (FK → agents)  │
│ name                    │
│ description             │
│ system_prompt           │
│ created_at              │
│ updated_at              │
│ version                 │
└─────────────────────────┘
```

### agent_context
```
┌──────────────────────────────┐
│ agent_context                │
├──────────────────────────────┤
│ agent_id (FK → agents)       │
│ document_id (FK → documents) │
│ created_at                   │
└──────────────────────────────┘
```

### workflow_step_agents
```
┌─────────────────────────────┐
│ workflow_step_agents        │
├─────────────────────────────┤
│ step_id (FK → steps)        │
│ agent_id (FK → agents)      │
│ agent_order                 │
│ created_at                  │
└─────────────────────────────┘
```

## Templates & Schemas

### prompt_templates
```
┌─────────────────────────┐
│ prompt_templates        │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ name                    │
│ template                │
│ variables               │
│ created_at              │
│ updated_at              │
│ version                 │
└─────────────────────────┘
```

### output_schemas
```
┌─────────────────────────┐
│ output_schemas          │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ name                    │
│ schema                  │
│ created_at              │
│ updated_at              │
│ version                 │
└─────────────────────────┘
```

### results
```
┌───────────────────────────────┐
│ results                       │
├───────────────────────────────┤
│ id (PK)                       │
│ user_id (FK → users)          │
│ agent_execution_id (FK)       │
│ output_schema_id (FK)         │
│ data                          │
│ created_at                    │
└───────────────────────────────┘
```

## Task & Project Management

### task_dependencies
```
┌─────────────────────────┐
│ task_dependencies       │
├─────────────────────────┤
│ task_id (FK → tasks)    │
│ depends_on_id (FK)      │
│ created_at              │
└─────────────────────────┘
```

### task_events
```
┌─────────────────────────┐
│ task_events             │
├─────────────────────────┤
│ id (PK)                 │
│ task_id (FK → tasks)    │
│ event_type              │
│ data                    │
│ created_at              │
└─────────────────────────┘
```

### tickets
```
┌─────────────────────────┐
│ tickets                 │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ title                   │
│ description             │
│ status                  │
│ priority                │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

### vertical_slices
```
┌─────────────────────────────┐
│ vertical_slices             │
├─────────────────────────────┤
│ id (PK)                     │
│ ticket_id (FK → tickets)    │
│ user_id (FK → users)        │
│ description                 │
│ acceptance_criteria         │
│ created_at                  │
│ updated_at                  │
└─────────────────────────────┘
```

### prds
```
┌─────────────────────────┐
│ prds                    │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ title                   │
│ content                 │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

### planning_sessions
```
┌─────────────────────────┐
│ planning_sessions       │
├─────────────────────────┤
│ id (PK)                 │
│ prd_id (FK → prds)      │
│ user_id (FK → users)    │
│ status                  │
│ output                  │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

## Monitoring & Analytics

### cost_records
```
┌─────────────────────────┐
│ cost_records            │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ agent_id (FK → agents)  │
│ task_id (FK → tasks)    │
│ model                   │
│ tokens                  │
│ cost                    │
│ created_at              │
└─────────────────────────┘
```

### llm_calls
```
┌─────────────────────────┐
│ llm_calls               │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ model                   │
│ input                   │
│ output                  │
│ tokens_used             │
│ cost                    │
│ duration_ms             │
│ created_at              │
└─────────────────────────┘
```

### token_usage
```
┌─────────────────────────┐
│ token_usage             │
├─────────────────────────┤
│ id (PK)                 │
│ model                   │
│ input_tokens            │
│ output_tokens           │
│ total_tokens            │
│ cost                    │
│ created_at              │
└─────────────────────────┘
```

### token_ledger
```
┌─────────────────────────────┐
│ token_ledger                │
├─────────────────────────────┤
│ id (PK)                     │
│ user_id (FK → users)        │
│ agent_execution_id (FK)     │
│ input_tokens                │
│ output_tokens               │
│ total_tokens                │
│ cost                        │
│ created_at                  │
└─────────────────────────────┘
```

## Automation & Scheduling

### schedules
```
┌─────────────────────────┐
│ schedules               │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ name                    │
│ cron_expression         │
│ action                  │
│ enabled                 │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

### triggers
```
┌─────────────────────────┐
│ triggers                │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ name                    │
│ event_type              │
│ condition               │
│ action                  │
│ enabled                 │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

## Refactoring & Code Management

### refactor_sessions
```
┌─────────────────────────┐
│ refactor_sessions       │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ description             │
│ status                  │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

### refactor_changes
```
┌──────────────────────────────────┐
│ refactor_changes                 │
├──────────────────────────────────┤
│ id (PK)                          │
│ session_id (FK → sessions)       │
│ file_path                        │
│ old_content                      │
│ new_content                      │
│ status                           │
│ created_at                       │
└──────────────────────────────────┘
```

### pr_merge_queue
```
┌─────────────────────────┐
│ pr_merge_queue          │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ pr_number               │
│ repo                    │
│ status                  │
│ priority                │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

## Miscellaneous

### decisions
```
┌─────────────────────────┐
│ decisions               │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ title                   │
│ description             │
│ options                 │
│ selected_option         │
│ rationale               │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

### messages
```
┌─────────────────────────┐
│ messages                │
├─────────────────────────┤
│ id (PK)                 │
│ from_agent (FK)         │
│ to_agent (FK)           │
│ task_id (FK → tasks)    │
│ content                 │
│ created_at              │
└─────────────────────────┘
```

### sessions
```
┌─────────────────────────┐
│ sessions                │
├─────────────────────────┤
│ id (PK)                 │
│ session_key             │
│ data                    │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

### context_store
```
┌──────────────────────────────┐
│ context_store                │
├──────────────────────────────┤
│ id (PK)                      │
│ session_id (FK → sessions)   │
│ key                          │
│ value                        │
│ created_at                   │
│ updated_at                   │
└──────────────────────────────┘
```

### step_documents
```
┌──────────────────────────────┐
│ step_documents               │
├──────────────────────────────┤
│ step_id (FK → steps)         │
│ document_id (FK → documents) │
│ created_at                   │
└──────────────────────────────┘
```

### router_requests
```
┌─────────────────────────────┐
│ router_requests             │
├─────────────────────────────┤
│ id (PK)                     │
│ session_id (FK)             │
│ agent_execution_id (FK)     │
│ request                     │
│ response                    │
│ created_at                  │
└─────────────────────────────┘
```

### auth_config
```
┌─────────────────────────┐
│ auth_config             │
├─────────────────────────┤
│ id (PK)                 │
│ provider                │
│ config                  │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

### system_state
```
┌─────────────────────────┐
│ system_state            │
├─────────────────────────┤
│ id (PK)                 │
│ key                     │
│ value                   │
│ updated_at              │
└─────────────────────────┘
```

## Version Tables

All versioned entities have corresponding `_versions` tables that reference the parent:

- **agents_versions** → agents
- **agent_modes_versions** → agent_modes
- **tools_versions** → tools
- **workflows_versions** → workflows
- **workflow_steps_versions** → workflow_steps
- **output_schemas_versions** → output_schemas
- **prompt_templates_versions** → prompt_templates

## Backup Tables

Some tables have backup versions:

- **agent_executions_backup**
- **room_sessions_backup**
- **rooms_backup**
- **pipelines_backup**

---

## Key Relationships Summary

### User-Centric (user_id FK)
Almost all major entities are owned by users:
- agents, tasks, tools, documents, workflows, chat_sessions, tickets, prds, etc.

### Workflow Execution Chain
```
users → workflow_collections → collection_runs → workflow_executions → agent_executions
```

### Agent-Task Relationship
```
users → tasks ← messages → agents
agents.current_task → tasks
```

### Tool Router Hierarchy
```
tool_routers → tool_router_modes → tool_router_mode_tools → tools
agents.router_id → tool_routers
```

### Room Collaboration
```
users → rooms → room_members → agents
rooms → room_sessions → agent_executions
```

### Context & Documents
```
users → chat_sessions → documents
agents → agent_context → documents
workflow_steps → step_documents → documents
```

### Execution Hierarchy
```
agent_executions (parent) → agent_executions (children)
agent_executions → execution_messages
agent_executions → execution_variables
```

---

**Total Tables:** 67 (including migrations and backup tables)
**Core Entity Types:** ~15 (users, agents, tasks, tools, workflows, documents, etc.)
**Junction/Relation Tables:** ~12 (agent_tools, room_members, step_documents, etc.)
**Versioning Tables:** 7
**Backup Tables:** 4
