# Nexor Database Entity Relationship Diagram

**Last Updated:** 2026-02-05
**Migration Version:** 065 (Unused tables cleanup)

This document describes the **active** database schema for the Nexor AI agent orchestration system. All tables listed here have corresponding Rust code in the application.

---

## Core Entities

### users
```
┌─────────────────┐
│ users           │
├─────────────────┤
│ id (PK)         │
│ email           │
│ password_hash   │
│ github_token    │
│ created_at      │
│ updated_at      │
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
│ system_prompt            │
│ model_provider           │
│ model_id                 │
│ model_max_tokens         │
│ model_temperature        │
│ router_id (FK)           │
│ output_schema_id (FK)    │
│ version                  │
│ created_at               │
│ updated_at               │
└──────────────────────────┘
```

### tasks
```
┌─────────────────────────┐
│ tasks                   │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ slice_id (FK)           │
│ title                   │
│ description             │
│ assigned_agent          │
│ status                  │
│ priority                │
│ context_files           │
│ metadata                │
│ retry_count             │
│ max_retries             │
│ last_error              │
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
│ display_name            │
│ description             │
│ parameters              │
│ created_at              │
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
│ summary                 │
│ doc_type                │
│ ref_tag                 │
│ tags                    │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

---

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
│ execution_mode           │
│ created_at               │
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
│ execution_mode                  │
│ agent_execution_mode            │
│ for_each_ref                    │
│ prompt_template_id (FK)         │
│ prompt_template                 │
│ output_schema_id (FK)           │
│ output_variable_name            │
│ interactive_agent_id (FK)       │
│ for_each_label_field            │
│ room_id (FK → rooms)            │
│ display_order                   │
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
│ execution_mode                  │
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
│ display_order                       │
│ execution_mode                      │
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
└─────────────────────────────────────┘
```

---

## Execution & Runtime

### collection_runs
```
┌─────────────────────────────────┐
│ collection_runs                 │
├─────────────────────────────────┤
│ id (PK)                         │
│ collection_id (FK → collections)│
│ user_id (FK → users)            │
│ status                          │
│ started_at                      │
│ completed_at                    │
│ error                           │
└─────────────────────────────────┘
```

### workflow_executions
```
┌─────────────────────────────────────┐
│ workflow_executions                 │
├─────────────────────────────────────┤
│ id (PK)                             │
│ collection_run_id (FK)              │
│ workflow_id (FK → workflows)        │
│ user_id (FK → users)                │
│ status                              │
│ started_at                          │
│ completed_at                        │
│ outputs                             │
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
│ is_interactive                          │
│ parent_agent_execution_id (FK → self)   │
│ system_prompt_rendered                  │
│ input                                   │
│ output                                  │
│ structured_output                       │
│ selected_mode_id (FK → agent_modes)     │
│ room_session_id (FK)                    │
│ speaker_order                           │
│ status                                  │
│ started_at                              │
│ completed_at                            │
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
│ tool_call_id                         │
│ input_tokens                         │
│ output_tokens                        │
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
│ variable_name                        │
│ variable_path                        │
│ value                                │
│ created_at                           │
└──────────────────────────────────────┘
```

---

## Chat & Sessions

### chat_sessions
```
┌─────────────────────────┐
│ chat_sessions           │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ mode_id                 │
│ title                   │
│ summary                 │
│ agent_id (FK → agents)  │
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
│ session_id (FK)          │
│ role                     │
│ content                  │
│ timestamp                │
└──────────────────────────┘
```

---

## Rooms & Collaboration

### rooms
```
┌─────────────────────────────────┐
│ rooms                           │
├─────────────────────────────────┤
│ id (PK)                         │
│ user_id (FK → users)            │
│ collection_id (FK)              │
│ name                            │
│ gatekeeper_enabled              │
│ gatekeeper_model_id             │
│ max_speakers_per_turn           │
│ max_turns                       │
│ tools_enabled                   │
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
│ display_name            │
│ role_description        │
│ display_order           │
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
│ current_turn            │
│ transcript_summary      │
│ started_at              │
│ completed_at            │
└─────────────────────────┘
```

---

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
│ system_prompt                  │
│ model_id                       │
│ is_active                      │
│ level                          │
│ created_at                     │
│ updated_at                     │
└────────────────────────────────┘
```

### tool_router_modes
```
┌─────────────────────────────────┐
│ tool_router_modes               │
├─────────────────────────────────┤
│ id (PK)                         │
│ router_id (FK → routers)        │
│ mode_key                        │
│ display_name                    │
│ description                     │
│ system_prompt                   │
│ temperature                     │
│ max_tokens                      │
│ append_to_agent_system_prompt   │
│ append_to_agent_tools           │
│ display_order                   │
│ created_at                      │
│ updated_at                      │
└─────────────────────────────────┘
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

---

## Agent Configuration

### agent_modes
```
┌─────────────────────────────┐
│ agent_modes                 │
├─────────────────────────────┤
│ id (PK)                     │
│ agent_id (FK → agents)      │
│ name                        │
│ system_prompt_suffix        │
│ temperature_override        │
│ model_override              │
│ tool_overrides              │
│ classifier_hint             │
│ created_at                  │
│ version                     │
└─────────────────────────────┘
```

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
│ execution_strategy          │
│ agent_order                 │
└─────────────────────────────┘
```

---

## Templates & Schemas

### prompt_templates
```
┌─────────────────────────┐
│ prompt_templates        │
├─────────────────────────┤
│ id (PK)                 │
│ user_id (FK → users)    │
│ name                    │
│ content                 │
│ created_at              │
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
│ name                          │
│ data                          │
│ created_at                    │
└───────────────────────────────┘
```

---

## Storage & Tracking

### token_ledger
```
┌─────────────────────────────┐
│ token_ledger                │
├─────────────────────────────┤
│ id (PK)                     │
│ user_id (FK → users)        │
│ agent_execution_id (FK)     │
│ model_id                    │
│ input_tokens                │
│ output_tokens               │
│ cost_usd                    │
│ created_at                  │
└─────────────────────────────┘
```

### context_store
```
┌──────────────────────────────┐
│ context_store                │
├──────────────────────────────┤
│ id (PK)                      │
│ session_id (FK)              │
│ source                       │
│ priority                     │
│ content                      │
│ metadata                     │
│ status                       │
│ created_at                   │
│ expires_at                   │
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
│ intent                      │
│ priority                    │
│ callback_hint               │
│ routed_tool                 │
│ routed_args                 │
│ is_async                    │
│ passdown                    │
│ chain                       │
│ status                      │
│ result                      │
│ created_at                  │
│ completed_at                │
└─────────────────────────────┘
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

---

## Infrastructure

### auth_config
```
┌─────────────────────────┐
│ auth_config             │
├─────────────────────────┤
│ id (PK)                 │
│ password_hash           │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

### pr_merge_queue
```
┌─────────────────────────┐
│ pr_merge_queue          │
├─────────────────────────┤
│ id (PK)                 │
│ repo_owner              │
│ repo_name               │
│ pr_number               │
│ queue_position          │
│ status                  │
│ conflict_info           │
│ error_message           │
│ created_at              │
│ updated_at              │
└─────────────────────────┘
```

---

## Versioning Tables

All versioned entities have corresponding `_versions` tables that track historical changes:

### agents_versions
Tracks agent configuration history (from migration 053).

### agent_modes_versions
Tracks agent mode history (from migration 053).

### tools_versions
Tracks tool definition history (from migration 053).

---

## Key Relationships Summary

### User-Centric (user_id FK)
Almost all major entities are owned by users:
- `agents`, `tasks`, `tools`, `documents`, `workflows`, `chat_sessions`, `rooms`

### Workflow Execution Chain
```
users → workflow_collections → collection_runs → workflow_executions → agent_executions
```

### Agent Execution Hierarchy
```
agent_executions (parent) → agent_executions (children)
agent_executions → execution_messages
agent_executions → execution_variables
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

### Chat System
```
users → chat_sessions → chat_messages
chat_sessions → agents (via mode_id/agent_id)
```

### Context & Documents
```
users → documents
agents → agent_context → documents
workflow_steps → step_documents → documents
```

---

## Migration History

**Recent Cleanup (Migration 065):**
Removed 17 unused tables that were created but never implemented:
- Task management: `task_events`, `task_dependencies`, `cost_records`, `messages`
- Project management: `tickets`, `vertical_slices`, `prds`, `planning_sessions`
- Automation: `schedules`, `triggers`
- Refactoring: `refactor_sessions`, `refactor_changes`
- Observability: `decisions`, `llm_calls`, `token_usage`
- Legacy: `sessions`, `system_state`

**Previous Cleanup:**
- Migration 061: Dropped pipeline system (6 tables)
- Migration 062: Dropped cluster system (3 tables)
- Migration 046: Dropped legacy `tool_calls` table

---

## Schema Statistics

**Active Tables:** 48
- Core entities: 5 (users, agents, tasks, tools, documents)
- Workflow system: 7
- Execution & runtime: 5
- Chat & sessions: 2
- Rooms: 3
- Tool router system: 4
- Agent configuration: 4
- Templates & schemas: 3
- Storage & tracking: 4
- Infrastructure: 2
- Versioning: 3
- Junction/relation tables: 6

**Total Storage Requirements:** ~48 active tables (down from 67 pre-cleanup)

**Database Engine:** PostgreSQL 14+
**Connection Pooling:** 10 connections (configurable via ENV)
