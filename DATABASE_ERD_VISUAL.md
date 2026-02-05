# Nexor Database ERD - Visual Diagram

## Core System Overview

```
                                    ┌─────────────────┐
                                    │     users       │
                                    │─────────────────│
                                    │ id (PK)         │
                                    │ username        │
                                    │ email           │
                                    │ password_hash   │
                                    │ github_token    │
                                    └────────┬────────┘
                                             │
                    ┌────────────────────────┼────────────────────────┐
                    │                        │                        │
                    ▼                        ▼                        ▼
         ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
         │     agents       │    │      tasks       │    │      tools       │
         │──────────────────│    │──────────────────│    │──────────────────│
         │ id (PK)          │    │ id (PK)          │    │ id (PK)          │
         │ user_id (FK)     │    │ user_id (FK)     │    │ user_id (FK)     │
         │ name             │    │ title            │    │ name             │
         │ system_prompt    │    │ description      │    │ description      │
         │ router_id (FK) ──┼─┐  │ status           │    │ schema           │
         │ current_task (FK)├─┼─▶│ priority         │    │ implementation   │
         └────────┬─────────┘ │  └──────────────────┘    └─────────┬────────┘
                  │           │                                     │
                  │           │                                     │
                  └───────────┼─────────────────────────────────────┘
                              │                                     │
                              ▼                                     │
                   ┌──────────────────┐                            │
                   │  tool_routers    │◀───────────────────────────┘
                   │──────────────────│
                   │ id (PK)          │
                   │ user_id (FK)     │
                   │ parent_router_id │
                   │ name             │
                   └────────┬─────────┘
                            │
                            ▼
                   ┌──────────────────────┐
                   │ tool_router_modes    │
                   │──────────────────────│
                   │ id (PK)              │
                   │ router_id (FK)       │
                   │ name                 │
                   └────────┬─────────────┘
                            │
                            ▼
                   ┌──────────────────────────┐
                   │ tool_router_mode_tools   │
                   │──────────────────────────│
                   │ mode_id (FK)             │
                   │ tool_id (FK)             │
                   └──────────────────────────┘
```

## Workflow & Execution Flow

```
        ┌─────────────────┐
        │     users       │
        └────────┬────────┘
                 │
                 ├──────────────────────────┐
                 │                          │
                 ▼                          ▼
     ┌──────────────────────┐   ┌──────────────────────┐
     │    workflows         │   │ workflow_collections │
     │──────────────────────│   │──────────────────────│
     │ id (PK)              │   │ id (PK)              │
     │ user_id (FK)         │   │ user_id (FK)         │
     │ name                 │   │ name                 │
     │ description          │   │ description          │
     └──────┬───────────────┘   └──────┬───────────────┘
            │                          │
            │    ┌─────────────────────┘
            │    │
            ▼    ▼
     ┌──────────────────────────┐
     │ collection_workflows     │
     │──────────────────────────│
     │ collection_id (FK)       │
     │ workflow_id (FK)         │
     │ workflow_order           │
     └──────────────────────────┘

            │
            │
            ▼
     ┌──────────────────────┐
     │ workflow_steps       │──────┐
     │──────────────────────│      │
     │ id (PK)              │      │
     │ workflow_id (FK)     │      │
     │ agent_id (FK) ───────┼──────┼─────┐
     │ room_id (FK)         │      │     │
     │ name                 │      │     │
     │ step_order           │      │     │
     └──────┬───────────────┘      │     │
            │                      │     │
            ▼                      │     │
     ┌──────────────────────────┐ │     │
     │ workflow_step_edges      │ │     │
     │──────────────────────────│ │     │
     │ from_step_id (FK)        │ │     │
     │ to_step_id (FK)          │ │     │
     │ condition                │ │     │
     └──────────────────────────┘ │     │
                                   │     │
                                   │     │
                                   │     ▼
                                   │  ┌──────────────────┐
                                   │  │     agents       │
                                   │  │──────────────────│
                                   │  │ id (PK)          │
                                   │  │ user_id (FK)     │
                                   │  │ name             │
                                   │  └──────────────────┘
                                   │
                                   ▼
                        ┌──────────────────────────┐
                        │ workflow_step_agents     │
                        │──────────────────────────│
                        │ step_id (FK)             │
                        │ agent_id (FK)            │
                        │ agent_order              │
                        └──────────────────────────┘
```

## Execution Runtime

```
     ┌──────────────────────┐
     │ workflow_collections │
     └──────┬───────────────┘
            │
            ▼
     ┌──────────────────────┐
     │  collection_runs     │
     │──────────────────────│
     │ id (PK)              │
     │ collection_id (FK)   │
     │ user_id (FK)         │
     │ status               │
     │ started_at           │
     └──────┬───────────────┘
            │
            ├─────────────────────────┐
            │                         │
            ▼                         ▼
     ┌──────────────────────┐  ┌─────────────────────┐
     │ workflow_executions  │  │ execution_variables │
     │──────────────────────│  │─────────────────────│
     │ id (PK)              │  │ id (PK)             │
     │ workflow_id (FK)     │  │ collection_run_id   │
     │ collection_run_id    │  │ workflow_exec_id    │
     │ user_id (FK)         │  │ step_execution_id   │
     │ status               │  │ key                 │
     │ started_at           │  │ value               │
     └──────┬───────────────┘  └─────────────────────┘
            │
            ▼
     ┌──────────────────────────────┐
     │   agent_executions           │
     │──────────────────────────────│
     │ id (PK)                      │
     │ agent_id (FK)                │
     │ workflow_step_id (FK)        │
     │ workflow_execution_id (FK)   │
     │ parent_agent_exec_id (FK) ◀──┼─┐ (self-reference)
     │ room_session_id (FK)         │ │
     │ selected_mode_id (FK)        │ │
     │ is_interactive               │ │
     │ input                        │ │
     │ output                       │ │
     │ status                       │ │
     │ started_at                   │ │
     └──────┬───────────────────────┘ │
            │                         │
            ├─────────────────────────┘
            │
            ├─────────────────┬─────────────────┐
            │                 │                 │
            ▼                 ▼                 ▼
     ┌─────────────────┐ ┌──────────────┐ ┌──────────────┐
     │ exec_messages   │ │ token_ledger │ │   results    │
     │─────────────────│ │──────────────│ │──────────────│
     │ id (PK)         │ │ id (PK)      │ │ id (PK)      │
     │ agent_exec_id   │ │ agent_exec_id│ │ agent_exec_id│
     │ role            │ │ input_tokens │ │ data         │
     │ content         │ │ output_tokens│ │ created_at   │
     └─────────────────┘ └──────────────┘ └──────────────┘
```

## Chat & Room System

```
        ┌─────────────────┐
        │     users       │
        └────────┬────────┘
                 │
                 ├────────────────────┬────────────────────┐
                 │                    │                    │
                 ▼                    ▼                    ▼
     ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
     │  chat_sessions   │  │      rooms       │  │     agents       │
     │──────────────────│  │──────────────────│  │──────────────────│
     │ id (PK)          │  │ id (PK)          │  │ id (PK)          │
     │ user_id (FK)     │  │ user_id (FK)     │  │ user_id (FK)     │
     │ agent_id (FK) ───┼──┼──────────────────┼─▶│ name             │
     │ title            │  │ collection_id    │  │ system_prompt    │
     └──────┬───────────┘  └──────┬───────────┘  └──────────────────┘
            │                     │
            │                     ├──────────────────┐
            │                     │                  │
            ▼                     ▼                  ▼
     ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
     │   documents      │  │  room_sessions   │  │  room_members    │
     │──────────────────│  │──────────────────│  │──────────────────│
     │ id (PK)          │  │ id (PK)          │  │ room_id (FK)     │
     │ user_id (FK)     │  │ room_id (FK)     │  │ agent_id (FK)    │
     │ session_id (FK)  │  │ status           │  │ role             │
     │ title            │  │ started_at       │  │ joined_at        │
     │ content          │  └──────────────────┘  └──────────────────┘
     └──────┬───────────┘
            │
            ▼
     ┌──────────────────┐
     │  agent_context   │
     │──────────────────│
     │ agent_id (FK)    │
     │ document_id (FK) │
     └──────────────────┘
```

## Task Management System

```
        ┌─────────────────┐
        │     users       │
        └────────┬────────┘
                 │
                 ├────────────────────┬────────────────────┐
                 │                    │                    │
                 ▼                    ▼                    ▼
     ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
     │      tasks       │  │     tickets      │  │       prds       │
     │──────────────────│  │──────────────────│  │──────────────────│
     │ id (PK)          │  │ id (PK)          │  │ id (PK)          │
     │ user_id (FK)     │  │ user_id (FK)     │  │ user_id (FK)     │
     │ title            │  │ title            │  │ title            │
     │ description      │  │ description      │  │ content          │
     │ status           │  │ status           │  └──────┬───────────┘
     └──────┬───────────┘  └──────┬───────────┘         │
            │                     │                     │
            ├──────────┐          │                     ▼
            │          │          │          ┌──────────────────────┐
            ▼          ▼          │          │  planning_sessions   │
     ┌──────────────────┐         │          │──────────────────────│
     │ task_dependencies│         │          │ id (PK)              │
     │──────────────────│         │          │ prd_id (FK)          │
     │ task_id (FK)     │         │          │ user_id (FK)         │
     │ depends_on_id ◀──┼─┐       │          │ status               │
     └──────────────────┘ │       │          └──────────────────────┘
                          │       │
     ┌──────────────────┐ │       │
     │   task_events    │ │       ▼
     │──────────────────│ │  ┌──────────────────┐
     │ id (PK)          │ │  │ vertical_slices  │
     │ task_id (FK) ────┼─┘  │──────────────────│
     │ event_type       │    │ id (PK)          │
     │ data             │    │ ticket_id (FK)   │
     └──────────────────┘    │ user_id (FK)     │
                             │ description      │
                             └──────────────────┘
```

## Agent Tools & Modes

```
     ┌──────────────────┐
     │     agents       │
     └────────┬─────────┘
              │
              ├──────────────────────┬──────────────────────┐
              │                      │                      │
              ▼                      ▼                      ▼
     ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
     │   agent_tools    │  │   agent_modes    │  │  agent_context   │
     │──────────────────│  │──────────────────│  │──────────────────│
     │ agent_id (FK)    │  │ id (PK)          │  │ agent_id (FK)    │
     │ tool_id (FK)     │  │ agent_id (FK)    │  │ document_id (FK) │
     └─────────┬────────┘  │ name             │  └──────────────────┘
               │           │ system_prompt    │
               │           └──────────────────┘
               │
               ▼
     ┌──────────────────┐
     │      tools       │
     │──────────────────│
     │ id (PK)          │
     │ user_id (FK)     │
     │ name             │
     │ schema           │
     │ implementation   │
     └──────────────────┘
```

## Templates & Schemas

```
        ┌─────────────────┐
        │     users       │
        └────────┬────────┘
                 │
                 ├────────────────────┬────────────────────┐
                 │                    │                    │
                 ▼                    ▼                    ▼
     ┌──────────────────────┐  ┌────────────────────┐  ┌──────────────────┐
     │  prompt_templates    │  │  output_schemas    │  │   documents      │
     │──────────────────────│  │────────────────────│  │──────────────────│
     │ id (PK)              │  │ id (PK)            │  │ id (PK)          │
     │ user_id (FK)         │  │ user_id (FK)       │  │ user_id (FK)     │
     │ name                 │  │ name               │  │ session_id (FK)  │
     │ template             │  │ schema             │  │ title            │
     │ variables            │  │ version            │  │ content          │
     └──────────────────────┘  └─────┬──────────────┘  └──────────────────┘
                                     │
                                     ├────────────┐
                                     │            │
                                     ▼            ▼
                            ┌────────────────┐  ┌────────────────┐
                            │    agents      │  │    results     │
                            │────────────────│  │────────────────│
                            │ output_schema  │  │ output_schema  │
                            │  _id (FK)      │  │  _id (FK)      │
                            └────────────────┘  └────────────────┘
```

## Monitoring & Cost Tracking

```
        ┌─────────────────┐
        │     users       │
        └────────┬────────┘
                 │
                 ├──────────────────┬──────────────────┬──────────────────┐
                 │                  │                  │                  │
                 ▼                  ▼                  ▼                  ▼
     ┌──────────────────┐ ┌──────────────────┐ ┌────────────┐ ┌─────────────┐
     │  cost_records    │ │   llm_calls      │ │token_ledger│ │token_usage  │
     │──────────────────│ │──────────────────│ │────────────│ │─────────────│
     │ id (PK)          │ │ id (PK)          │ │ id (PK)    │ │ id (PK)     │
     │ user_id (FK)     │ │ user_id (FK)     │ │ user_id    │ │ model       │
     │ agent_id (FK)    │ │ model            │ │ agent_exec │ │ input_tokens│
     │ task_id (FK)     │ │ input            │ │  _id (FK)  │ │ output_tok  │
     │ model            │ │ output           │ │ cost       │ │ total_tok   │
     │ tokens           │ │ tokens_used      │ └────────────┘ │ cost        │
     │ cost             │ │ cost             │                └─────────────┘
     └──────────────────┘ │ duration_ms      │
                          └──────────────────┘
```

## Automation & Refactoring

```
        ┌─────────────────┐
        │     users       │
        └────────┬────────┘
                 │
                 ├──────────────────┬──────────────────┬──────────────────┐
                 │                  │                  │                  │
                 ▼                  ▼                  ▼                  ▼
     ┌──────────────────┐ ┌──────────────────┐ ┌────────────────┐ ┌────────────┐
     │    schedules     │ │    triggers      │ │ refactor_sess  │ │pr_merge_que│
     │──────────────────│ │──────────────────│ │────────────────│ │────────────│
     │ id (PK)          │ │ id (PK)          │ │ id (PK)        │ │ id (PK)    │
     │ user_id (FK)     │ │ user_id (FK)     │ │ user_id (FK)   │ │ user_id    │
     │ name             │ │ name             │ │ description    │ │ pr_number  │
     │ cron_expression  │ │ event_type       │ │ status         │ │ repo       │
     │ action           │ │ condition        │ └────┬───────────┘ │ status     │
     │ enabled          │ │ action           │      │             └────────────┘
     └──────────────────┘ │ enabled          │      │
                          └──────────────────┘      │
                                                    ▼
                                         ┌──────────────────────┐
                                         │  refactor_changes    │
                                         │──────────────────────│
                                         │ id (PK)              │
                                         │ session_id (FK)      │
                                         │ file_path            │
                                         │ old_content          │
                                         │ new_content          │
                                         │ status               │
                                         └──────────────────────┘
```

## Versioning System

```
     ┌──────────────────┐           ┌──────────────────────┐
     │     agents       │           │  agents_versions     │
     │──────────────────│           │──────────────────────│
     │ id (PK)          │◀──────────│ id (FK → agents)     │
     │ version          │           │ version              │
     └──────────────────┘           │ (snapshot data)      │
                                    └──────────────────────┘

     ┌──────────────────┐           ┌──────────────────────┐
     │   agent_modes    │           │ agent_modes_versions │
     │──────────────────│           │──────────────────────│
     │ id (PK)          │◀──────────│ id (FK)              │
     │ version          │           │ version              │
     └──────────────────┘           └──────────────────────┘

     ┌──────────────────┐           ┌──────────────────────┐
     │      tools       │           │   tools_versions     │
     │──────────────────│           │──────────────────────│
     │ id (PK)          │◀──────────│ id (FK)              │
     │ version          │           │ version              │
     └──────────────────┘           └──────────────────────┘

     ┌──────────────────┐           ┌──────────────────────────┐
     │   workflows      │           │  workflows_versions      │
     │──────────────────│           │──────────────────────────│
     │ id (PK)          │◀──────────│ id (FK)                  │
     │ version          │           │ version                  │
     └──────────────────┘           └──────────────────────────┘

     ┌──────────────────────┐       ┌──────────────────────────────┐
     │  workflow_steps      │       │ workflow_steps_versions      │
     │──────────────────────│       │──────────────────────────────│
     │ id (PK)              │◀──────│ id (FK)                      │
     │ version              │       │ version                      │
     └──────────────────────┘       └──────────────────────────────┘

     ┌──────────────────────┐       ┌──────────────────────────────┐
     │  output_schemas      │       │ output_schemas_versions      │
     │──────────────────────│       │──────────────────────────────│
     │ id (PK)              │◀──────│ id (FK)                      │
     │ version              │       │ version                      │
     └──────────────────────┘       └──────────────────────────────┘

     ┌──────────────────────┐       ┌──────────────────────────────┐
     │  prompt_templates    │       │ prompt_templates_versions    │
     │──────────────────────│       │──────────────────────────────│
     │ id (PK)              │◀──────│ id (FK)                      │
     │ version              │       │ version                      │
     └──────────────────────┘       └──────────────────────────────┘
```

## Legend

```
─────▶   One-to-Many relationship (FK points to PK)
◀─────   Many-to-One relationship (visual perspective)
├─────   Split/Branch in relationship
│        Connection line
┌─┐      Table border
(PK)     Primary Key
(FK)     Foreign Key
```

---

## Key Relationship Patterns

1. **User Ownership**: Almost all entities have `user_id (FK → users.id)`
2. **Execution Hierarchy**: `collection_runs → workflow_executions → agent_executions`
3. **Self-References**: `agent_executions.parent_agent_execution_id`, `tool_routers.parent_router_id`
4. **Many-to-Many**: Junction tables like `agent_tools`, `room_members`, `step_documents`
5. **Versioning**: All versioned tables reference parent with `id (FK)` and track `version` number
