# Agent System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        USER MESSAGE                                 │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     AGENT MODE ROUTER                               │
│                  (src/server/agent_mode.rs)                          │
│                                                                     │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐  ┌──────────────┐  │
│  │   Home   │  │ Planning │  │ Agent Builder  │  │    Decomp    │  │
│  │ all tools│  │ docs only│  │ agents + files │  │  tickets +   │  │
│  │ no hist  │  │ 30 msgs  │  │   20 msgs      │  │  pipelines   │  │
│  └──────────┘  └──────────┘  └───────────────┘  └──────────────┘  │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    ORCHESTRATOR LOOP                                 │
│               (src/server/orchestrator.rs)                           │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  System Prompt (from mode or role)                          │   │
│  │  + Required Reading (loaded files)                          │   │
│  │  + History (if SessionScoped)                               │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                               │                                     │
│           ┌───────────────────┼──────────────────┐                  │
│           ▼                   ▼                  ▼                  │
│     ┌──────────┐     ┌──────────────┐    ┌────────────┐            │
│     │ LLM Call │────▶│ Tool Use?    │───▶│ Text Reply │            │
│     │ (Sonnet) │     │              │    │ → return   │            │
│     └──────────┘     └──────┬───────┘    └────────────┘            │
│                             │ yes                                   │
│                             ▼                                       │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    TOOL DISPATCH                              │  │
│  │               (src/server/tools.rs)                           │  │
│  │                                                               │  │
│  │  Server Tools (30+):              Execution Tools (11):       │  │
│  │  ┌─────────────────────┐          ┌────────────────────┐     │  │
│  │  │ create_agent        │          │ read_file          │     │  │
│  │  │ assign_task         │          │ edit_file          │     │  │
│  │  │ create_pipeline     │          │ write_file         │     │  │
│  │  │ read_file (smart)   │          │ search_files       │     │  │
│  │  │ search_files        │          │ list_files         │     │  │
│  │  │ list_files          │          │ git_status/diff/   │     │  │
│  │  │ ...                 │          │   add/commit/branch│     │  │
│  │  └─────────────────────┘          │ run_tests          │     │  │
│  │                                   │ run_command        │     │  │
│  │                                   └────────────────────┘     │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                             │                                       │
│                             ▼                                       │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  GUARDRAILS                                                   │  │
│  │  • Result truncation: 10K chars max per tool result           │  │
│  │  • Compact JSON (no pretty-print)                             │  │
│  │  • Context budget: break at 480K chars (~120K tokens)         │  │
│  │  • 200ms delay between rounds                                 │  │
│  │  • RetryingProvider with exponential backoff                  │  │
│  │  • Tool call persistence to DB                                │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                             │                                       │
│                     loop back to LLM                                │
│                    (max 10 rounds)                                   │
└─────────────────────────────────────────────────────────────────────┘

                    ┌─────────────────┐
                    │  assign_task    │
                    │  triggers...    │
                    └────────┬────────┘
                             ▼

┌─────────────────────────────────────────────────────────────────────┐
│                     AGENT EXECUTION                                 │
│              (src/agents/executor.rs)                                │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │  ROLE CONTEXT (src/agents/roles.rs)                           │ │
│  │                                                                │ │
│  │  System Prompt (from src/prompts/*.txt)                        │ │
│  │  + Required Reading (CONVENTIONS.md, ticket files, etc.)       │ │
│  │  + Allowed Tools (filtered per-task)                           │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  INTRODUCTION PROTOCOL (all agents do this first):                  │
│  ┌──────┐    ┌──────────────┐    ┌──────────┐    ┌────────────┐   │
│  │ 1.   │───▶│ 2.           │───▶│ 3.       │───▶│ 4.         │   │
│  │list_ │    │search_files  │    │read_file │    │edit_file   │   │
│  │files │    │(grep for     │    │(targeted │    │(surgical   │   │
│  │(map) │    │ keywords)    │    │ reads)   │    │ edits)     │   │
│  └──────┘    └──────────────┘    └──────────┘    └────────────┘   │
│                                                                     │
│  TIERS:                                                             │
│  ┌──────────────────┐  ┌─────────────────┐  ┌──────────────────┐  │
│  │  Orchestrator    │  │    Worker        │  │    Utility       │  │
│  │  Tier 2 (Opus)   │  │  Tier 1 (Sonnet) │  │  Tier 0 (Haiku)  │  │
│  │  Plans, delegates│  │  Codes, commits  │  │  Lint, format    │  │
│  │  10-15 rounds    │  │  5-10 rounds     │  │  1-3 rounds      │  │
│  │  Can delegate ↓  │  │  Can delegate ↓  │  │  Cannot delegate │  │
│  └────────┬─────────┘  └────────┬─────────┘  └──────────────────┘  │
│           │                     │                                   │
│           ▼                     ▼                                   │
│      Workers, Utilities    Utilities only                           │
│                                                                     │
│  EXECUTION LAYER (src/execution/):                                  │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────────┐  │
│  │  FileOps   │ │  GitOps    │ │ TestRunner │ │   Sandbox      │  │
│  │ read_file  │ │ status     │ │ run_tests  │ │ exec_shell     │  │
│  │ write_file │ │ diff/add   │ │ run_specif │ │ (sandboxed)    │  │
│  │ list_dir   │ │ commit     │ │            │ │                │  │
│  │ (path val) │ │ branch     │ │            │ │                │  │
│  └────────────┘ └────────────┘ └────────────┘ └────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘

edit_file flow:
┌──────────────────────────────────────────────────────────┐
│  Agent calls:                                            │
│  edit_file({                                             │
│    path: "src/server/tools.rs",                          │
│    old_string: "fn handle_error() {\n    todo!()\n}",    │
│    new_string: "fn handle_error() {\n    Err(...)\n}"    │
│  })                                                      │
│                          │                               │
│                          ▼                               │
│  1. FileOps::read_file → get current content             │
│  2. Count matches of old_string                          │
│     ├─ 0 matches → error "not found"                    │
│     ├─ 2+ matches → error "not unique, add context"     │
│     └─ 1 match → replace                                │
│  3. FileOps::write_file → save edited content            │
│  4. Return { line_start, line_end, preview }             │
└──────────────────────────────────────────────────────────┘
```
