# Database Schema (ERD)

## Entity-Relationship Diagram

```
┌─────────────┐
│    users    │ (Root - Multi-tenant anchor)
│─────────────│
│ id (PK)     │
│ email       │
│ password    │
│ github_*    │
└──────┬──────┘
       │
       │ (All tables reference user_id for tenant isolation)
       │
       ├──────────────────────────────────────────────────────────────┐
       │                                                              │
       │                                                              │
┌──────▼──────┐         ┌──────────────┐         ┌────────────────┐│
│   agents    │◄────────┤ agent_tools  │────────►│     tools      ││
│─────────────│         │ (join table) │         │────────────────││
│ id (PK)     │         └──────────────┘         │ id (PK)        ││
│ user_id (FK)│                                  │ user_id (FK)   ││
│ tier        │         ┌──────────────┐         │ name           ││
│ persona_*   │◄────────┤agent_context │         │ display_name   ││
│ model_*     │         │ (join table) │         │ description    ││
│ status      │         └──────┬───────┘         │ parameters     ││
│ current_task├──┐              │                 └────────┬───────┘│
└──────┬──────┘  │              │                          │        │
       │         │       ┌──────▼──────┐                   │        │
       │         │       │  documents  │                   │        │
       │         │       │─────────────│                   │        │
       │         │       │ id (PK)     │                   │        │
       │         │       │ user_id (FK)│         ┌─────────▼────────▼─┐
       │         │       │ session_id  │         │   tool_routers     │
       │         │       │ title       │         │────────────────────│
       │         │       │ content     │         │ id (PK)            │
       │         │       │ doc_type    │         │ user_id (FK)       │
       │         │       │ tags[]      │         │ system_prompt      │
       │         │       └──────┬──────┘         │ model_id           │
       │         │              │                 └─────────┬──────────┘
       │         │              │                           │
       │         │              │                 ┌─────────▼──────────┐
       │         │              │                 │ tool_router_tools  │
       │         │              │                 │   (join table)     │
       │         │              │                 └────────────────────┘
       │         │              │
       │    ┌────▼────┐         │         ┌──────────────┐
       │    │  tasks  │         │         │agent_modes   │
       │    │─────────│         │         │──────────────│
       │    │ id (PK) │         │         │ id (PK)      │
       │    │ user_id │         │         │ agent_id (FK)├──┐
       │    │ title   │         │         │ name         │  │
       │    │ status  │         │         │ prompt_suffix│  │
       │    │ priority│         │         │ temp_override│  │
       │    └─────────┘         │         │ tool_override│  │
       │                        │         └──────────────┘  │
       │                        │                           │
       │                        │                           │
       ├────────────────────────┴───────────────────────────┘
       │
       │  WORKFLOW & EXECUTION SYSTEM
       │
       ├──────────────────────────────────────────────────────────────┐
       │                                                              │
┌──────▼────────┐                                                    │
│   workflows   │                                                    │
│───────────────│                                                    │
│ id (PK)       │                                                    │
│ user_id (FK)  │                                                    │
│ name          │                                                    │
└──────┬────────┘                                                    │
       │                                                             │
       │ 1:N                                                         │
       │                                                             │
┌──────▼─────────────┐       ┌─────────────────┐                    │
│  workflow_steps    │◄──────┤ step_documents  │                    │
│────────────────────│       │  (join table)   │                    │
│ id (PK)            │       └────────┬─────┬──┘                    │
│ workflow_id (FK)   │                │     │                       │
│ agent_id (FK)      ├────────────────┘     └──────┐                │
│ prompt_template_id │                              │                │
│ output_schema_id   │       ┌──────────────────┐   │                │
│ interactive_agent  │       │ prompt_templates │   │                │
│ room_id (FK)       │       │──────────────────│   │                │
│ execution_mode     │       │ id (PK)          │   │                │
│ for_each_ref       │       │ user_id (FK)     │   │                │
│ display_order      │       │ name             │   │                │
└──────┬─────────────┘       │ content          │   │                │
       │                     └──────────────────┘   │                │
       │ N:M (DAG)                                  │                │
       │                     ┌──────────────────┐   │                │
┌──────▼─────────────┐       │ output_schemas   │   │                │
│workflow_step_edges │       │──────────────────│   │                │
│────────────────────│       │ id (PK)          │   │                │
│ from_step_id (FK)  │       │ user_id (FK)     │   │                │
│ to_step_id (FK)    │       │ name             │   │                │
└────────────────────┘       │ schema (JSONB)   │   │                │
                             └──────────────────┘   │                │
                                                    │                │
                                       documents◄───┘                │
                                                                     │
       ├─────────────────────────────────────────────────────────────┘
       │
       │  PIPELINE EXECUTION SYSTEM
       │
┌──────▼────────┐
│   pipelines   │
│───────────────│
│ id (PK)       │
│ user_id (FK)  │
│ name          │
└──────┬────────┘
       │
       ├──────────────────────┬─────────────────────┐
       │                      │                     │
┌──────▼─────────┐    ┌───────▼────────┐    ┌──────▼──────┐
│pipeline_stages │    │ pipeline_runs  │    │    rooms    │
│────────────────│    │────────────────│    │─────────────│
│ pipeline_id(FK)│    │ id (PK)        │    │ id (PK)     │
│ stage_number   │    │ pipeline_id(FK)│    │ user_id (FK)│
│ agent_id (FK)  │    │ user_id (FK)   │    │ pipeline_id │
│ role           │    │ status         │    │ name        │
└────────────────┘    │ current_stage  │    │ gatekeeper_*│
                      │ stage_outputs  │    │ max_*       │
                      └───────┬────────┘    └──────┬──────┘
                              │                    │
                              │ 1:N                │ N:M
                              │             ┌──────▼────────┐
                      ┌───────▼────────┐   │ room_members  │
                      │stage_executions│   │ (join table)  │
                      │────────────────│   │───────────────│
                      │ id (PK)        │   │ room_id (FK)  │
                      │ run_id (FK)    │   │ agent_id (FK) │
                      │ agent_id       │   │ display_name  │
                      │ stage_number   │   │ role_desc     │
                      │ status         │   └───────────────┘
                      │ rendered_prompt│
                      │ output         │           │
                      │ structured_out │           │ 1:N
                      │ input_tokens   │    ┌──────▼────────┐
                      │ output_tokens  │    │ room_sessions │
                      └───────┬────────┘    │───────────────│
                              │             │ id (PK)       │
                              │ 1:N         │ room_id (FK)  │
                              │             │ run_id (FK)   │
                      ┌───────▼───────────┐ │ status        │
                      │agent_executions   │ │ current_turn  │
                      │───────────────────│ └──────┬────────┘
                      │ id (PK)           │        │
                      │ stage_exec_id (FK)│◄───────┘
                      │ agent_id (FK)     │
                      │ workflow_step_id  │
                      │ parent_exec_id ◄──┼─┐ (self-ref)
                      │ room_session_id   │ │
                      │ is_interactive    │ │
                      │ system_prompt     │ │
                      │ input/output      │ │
                      │ structured_output │ │
                      │ status            │ │
                      │ input/out_tokens  │ │
                      │ cost_usd          │ │
                      └───────┬───────────┘ │
                              │             │
                              │ 1:N         │
                              │             │
                      ┌───────▼───────────┐ │
                      │execution_messages │ │
                      │───────────────────│ │
                      │ id (PK)           │ │
                      │ agent_exec_id (FK)├─┘
                      │ role              │
                      │ content           │
                      │ tool_call_id      │
                      │ input/out_tokens  │
                      └───────────────────┘
                              │
                              │
                      ┌───────▼───────┐
                      │    results    │
                      │───────────────│
                      │ id (PK)       │
                      │ agent_exec_id │
                      │ output_sch_id │
                      │ name          │
                      │ data (JSONB)  │
                      └───────────────┘

       │
       │  SESSION & COMMUNICATION SYSTEM
       │
┌──────▼────────┐
│ chat_sessions │
│───────────────│
│ id (PK)       │
│ user_id (FK)  │
│ agent_id (FK) │
│ mode_id       │
│ title         │
└──────┬────────┘
       │
       ├──────────────────┬─────────────────┐
       │                  │                 │
┌──────▼────────┐  ┌──────▼──────┐  ┌──────▼────────┐
│ chat_messages │  │ tool_calls  │  │context_store  │
│───────────────│  │─────────────│  │───────────────│
│ id (PK)       │  │ id (PK)     │  │ id (PK)       │
│ user_id (FK)  │  │ session_id  │  │ session_id(FK)│
│ session_id    │  │ message_id  │  │ content_type  │
│ role          │  │ round       │  │ content       │
│ content       │  │ tool_name   │  │ priority      │
│ timestamp     │  │ input/output│  │ created_at    │
└───────────────┘  │ latency_ms  │  └───────────────┘
                   └─────────────┘
                                    ┌───────────────┐
                                    │router_requests│
                                    │───────────────│
                                    │ id (PK)       │
                                    │ session_id(FK)│
                                    │ user_input    │
                                    │ selected_tool │
                                    │ confidence    │
                                    └───────────────┘

       │
       │  MONITORING & ANALYTICS
       │
┌──────▼─────────┐
│ token_ledger   │
│────────────────│
│ id (PK)        │
│ user_id (FK)   │
│ source_type    │
│ source_id      │
│ model_id       │
│ input_tokens   │
│ output_tokens  │
│ cost_usd       │
│ created_at     │
└────────────────┘
```

## Key Relationships Summary

### Document Context (The Missing Link!)
```
agents ──┬── agent_context ──► documents  (NOT USED in execution ❌)
         │
         └── agent_executions ──► workflow_steps ──► step_documents ──► documents  (USED ✅)
```

### Execution Hierarchy
```
pipeline_runs
  └── stage_executions
      └── agent_executions (can have workflow_step_id)
          ├── execution_messages
          └── results (structured outputs)
```

### Agent Configuration
```
agents
  ├── agent_tools ──► tools (what tools can agent use)
  ├── agent_context ──► documents (knowledge base - NOT LOADED)
  └── agent_modes (behavioral variants)
```

### Workflow DAG
```
workflows
  └── workflow_steps (nodes)
      ├── workflow_step_edges (defines order)
      ├── step_documents ──► documents (step context - LOADED ✅)
      ├── prompt_template_id ──► prompt_templates
      └── output_schema_id ──► output_schemas
```

### Room Collaboration
```
rooms
  ├── room_members ──► agents
  └── room_sessions
      └── agent_executions (speaker_order tracks turn)
```

## The Problem

**Agent Context Documents are stored but never loaded during execution.**

When `compose_prompt()` is called in `dag_executor.rs`:
- ✅ Loads `step_documents` (documents attached to workflow steps)
- ❌ Does NOT load `agent_context` (documents attached to agents)

This means if you attach documents to an agent via `PUT /agents/:id/context`, they just sit in the database unused.
