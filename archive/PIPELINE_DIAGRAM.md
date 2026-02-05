# Router Modes Pipeline

```
╔══════════════════════════════════════════════════════════════════════════════════════╗
║                                                                                      ║
║   "Help me debug this React component that keeps re-rendering"                       ║
║                                                                                      ║
╚══════════════════════════════════════════╤═══════════════════════════════════════════╝
                                           │
                                           │  WebSocket / HTTP
                                           │
                                           ▼
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  LAYER 3: APPLICATION                                                                ┃
┃  chat_consumer.rs │ workflow_executor.rs │ room_executor.rs                           ┃
┃                                                                                      ┃
┃    orchestrator.execute_agent(agent_id, input, history, sink, recorder, cancel)       ┃
┃                                                                                      ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
                                │
                                ▼
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  LAYER 2: AGENT ORCHESTRATOR                                                         ┃
┃ ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐  ┃
┃                                                                                      ┃
┃ │  ┌──────────────────────────────────────────────────────────────────────────┐   │  ┃
┃    │  STAGE 1: LOAD AGENT                                                    │      ┃
┃ │  │                                                                         │   │  ┃
┃    │  ┌─────────────┐     SELECT * FROM agents                               │      ┃
┃ │  │  │  PostgreSQL │────────────────────────────────────┐                   │   │  ┃
┃    │  └─────────────┘                                    │                   │      ┃
┃ │  │                                                     ▼                   │   │  ┃
┃    │                                          ┌─────────────────────┐        │      ┃
┃ │  │                                          │ AgentRow            │        │   │  ┃
┃    │                                          │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │        │      ┃
┃ │  │                                          │ name: "CodeBot"     │        │   │  ┃
┃    │                                          │ model: Sonnet       │        │      ┃
┃ │  │                                          │ temp: 0.7           │        │   │  ┃
┃    │                                          │ router_id: r1r2...  │──┐     │      ┃
┃ │  │                                          └─────────────────────┘  │     │   │  ┃
┃    └───────────────────────────────────────────────────────────────┬────┘─────┘      ┃
┃ │                                                                  │             │  ┃
┃                                                       Has router?  │                 ┃
┃ │                                            ┌─────── YES ─────────┘             │  ┃
┃                                              │                                       ┃
┃ │                                            ▼                                   │  ┃
┃    ┌──────────────────────────────────────────────────────────────────────────┐       ┃
┃ │  │  STAGE 2: LOAD ROUTER + MODES                                          │   │  ┃
┃    │                                                                         │       ┃
┃ │  │  ┌─────────────┐     SELECT * FROM tool_routers                        │   │  ┃
┃    │  │  PostgreSQL │──────────────────────────────────┐                     │       ┃
┃ │  │  └─────────────┘     SELECT * FROM                │                    │   │  ┃
┃    │        │              tool_router_modes            │                    │       ┃
┃ │  │        └──────────────────────────────────┐       │                    │   │  ┃
┃    │                                           │       │                    │       ┃
┃ │  │                                           ▼       ▼                    │   │  ┃
┃    │                           ┌───────────────────────────────────┐         │       ┃
┃ │  │                           │ ToolRouterRow                     │         │   │  ┃
┃    │                           │ ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄ │         │       ┃
┃ │  │                           │ name: "CodeBot Task Router"       │         │   │  ┃
┃    │                           │ model: Haiku                      │         │       ┃
┃ │  │                           │ prompt: "You are a conversation   │         │   │  ┃
┃    │                           │          classifier..."           │         │       ┃
┃ │  │                           └───────────────────────────────────┘         │   │  ┃
┃    │                                                                         │       ┃
┃ │  │                           ┌────────────┐┌────────────┐┌────────────┐   │   │  ┃
┃    │                           │  coding     ││  research  ││    chat    │   │       ┃
┃ │  │                           │  temp: 0.3  ││  temp: 0.5 ││  temp: 0.9│   │   │  ┃
┃    │                           │  append: T  ││  append: T ││  append: F│   │       ┃
┃ │  │                           │  tools: RPL ││  tools: RPL││  tools: RPL   │   │  ┃
┃    │                           └──────┬──────┘└────────────┘└────────────┘   │       ┃
┃ │  └──────────────────────────────────┼──────────────────────────────────────┘   │  ┃
┃                                       │                                              ┃
┃ │                                     │ 3 modes available                        │  ┃
┃                                       ▼                                              ┃
┃ │  ┌──────────────────────────────────────────────────────────────────────────┐   │  ┃
┃    │  STAGE 3: CLASSIFY (Router LLM Call)                                    │      ┃
┃ │  │                                                                         │   │  ┃
┃    │                     ┌──────────────────────────────────────────┐         │      ┃
┃ │  │                     │  RouterStrategy                          │         │   │  ┃
┃    │                     │                                          │         │      ┃
┃ │  │                     │  system: "You are a conversation         │         │   │  ┃
┃    │                     │           classifier..."                 │         │      ┃
┃ │  │                     │  model:  Haiku                           │         │   │  ┃
┃    │                     │  temp:   0.0 (deterministic)             │         │      ┃
┃ │  │                     │  tools:  [] (none)                       │         │   │  ┃
┃    │                     │  rounds: 1 (single shot)                 │         │      ┃
┃ │  │                     └────────────────┬─────────────────────────┘         │   │  ┃
┃    │                                      │                                  │      ┃
┃ │  │                                      ▼                                  │   │  ┃
┃    │                     ┌──────────────────────────────────────────┐         │      ┃
┃ │  │                     │             HAIKU LLM                    │         │   │  ┃
┃    │                     │                                          │         │      ┃
┃ │  │                     │  IN:  "## History:\n...\n## Input:\n     │         │   │  ┃
┃    │                     │        Help me debug...\n## Modes:\n     │         │      ┃
┃ │  │                     │        - coding: ...\n- research: ...\n  │         │   │  ┃
┃    │                     │        - chat: ...\nOutput ONLY the      │         │      ┃
┃ │  │                     │        mode key."                        │         │   │  ┃
┃    │                     │                                          │         │      ┃
┃ │  │                     │  OUT: "coding"                           │         │   │  ┃
┃    │                     │        ┄┄┄┄┄┄┄                           │         │      ┃
┃ │  │                     │  ~200 input tokens, ~5 output tokens     │         │   │  ┃
┃    │                     └────────────────┬─────────────────────────┘         │      ┃
┃ │  │                                      │                                  │   │  ┃
┃    └──────────────────────────────────────┼──────────────────────────────────┘      ┃
┃ │                                         │                                      │  ┃
┃                                selected = "coding"                                   ┃
┃ │                                         │                                      │  ┃
┃                                           ▼                                          ┃
┃ │  ┌──────────────────────────────────────────────────────────────────────────┐   │  ┃
┃    │  STAGE 4: LOAD MODE TOOLS                                               │      ┃
┃ │  │                                                                         │   │  ┃
┃    │  ┌─────────────┐     SELECT t.* FROM tools t                            │      ┃
┃ │  │  │  PostgreSQL │──── JOIN tool_router_mode_tools ──┐                    │   │  ┃
┃    │  └─────────────┘     WHERE mode_id = 'm001-...'    │                    │      ┃
┃ │  │                                                    ▼                    │   │  ┃
┃    │                      ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐  │      ┃
┃ │  │                      │   bash   │ │read_file │ │edit_file │ │search_ │  │   │  ┃
┃    │                      │          │ │          │ │          │ │  code  │  │       ┃
┃ │  │                      └──────────┘ └──────────┘ └──────────┘ └────────┘  │   │  ┃
┃    │                                                                         │      ┃
┃ │  │                      4 tools (not 50)                                   │   │  ┃
┃    └─────────────────────────────────────────────────────────────────────────┘      ┃
┃ │                                         │                                      │  ┃
┃                                           ▼                                          ┃
┃ │  ┌──────────────────────────────────────────────────────────────────────────┐   │  ┃
┃    │  STAGE 5: RESOLVE FINAL CONFIG                                          │      ┃
┃ │  │                                                                         │   │  ┃
┃    │   SYSTEM PROMPT                          TOOLS                          │      ┃
┃ │  │   ┄┄┄┄┄┄┄┄┄┄┄┄┄                         ┄┄┄┄┄                         │   │  ┃
┃    │   append_to_agent = true                  append_to_agent = false        │      ┃
┃ │  │                                                                         │   │  ┃
┃    │   ┌─────────────────────────┐             ┌──────────────────────────┐   │      ┃
┃ │  │   │ AGENT: "You are a      │             │ AGENT: [read_file,       │   │   │  ┃
┃    │   │  helpful AI coding     │             │  write_file, bash, git,  │   │       ┃
┃ │  │   │  assistant..."         │             │  edit_file, search_code, │   │   │  ┃
┃    │   ├─────────────────────────┤             │  web_search, web_fetch,  │   │      ┃
┃ │  │   │           +            │             │  create_pr, list_issues, │   │   │  ┃
┃    │   ├─────────────────────────┤             │  ... 40 more]           │   │       ┃
┃ │  │   │ MODE: "Focus on code   │             │             ╳ DISCARDED │   │   │  ┃
┃    │   │  quality and best      │             └──────────────────────────┘   │       ┃
┃ │  │   │  practices..."         │                                            │   │  ┃
┃    │   └─────────────────────────┘             ┌──────────────────────────┐   │      ┃
┃ │  │              │                            │ MODE: [bash, read_file,  │   │   │  ┃
┃    │              ▼                            │  edit_file, search_code] │   │       ┃
┃ │  │   ┌─────────────────────────┐             │             ✓ USED      │   │   │  ┃
┃    │   │ FINAL: "You are a      │             └──────────────────────────┘   │       ┃
┃ │  │   │  helpful AI coding     │                            │               │   │  ┃
┃    │   │  assistant...\n\n      │                            ▼               │       ┃
┃ │  │   │  Focus on code quality │             ┌──────────────────────────┐   │   │  ┃
┃    │   │  and best practices.." │             │ FINAL: [bash, read_file, │   │       ┃
┃ │  │   └────────────┬──────────┘             │  edit_file, search_code] │   │   │  ┃
┃    │                │                         └────────────┬─────────────┘   │      ┃
┃ │  │                │     ┌──────────────┐                 │                 │   │  ┃
┃    │                │     │ temp: 0.3    │                 │                 │       ┃
┃ │  │                │     │ model: Sonnet│                 │                 │   │  ┃
┃    │                │     │ max_tok: 8000│                 │                 │       ┃
┃ │  │                │     └──────┬───────┘                 │                 │   │  ┃
┃    │                │            │                         │                 │       ┃
┃ │  │                └────────────┼─────────────────────────┘                 │   │  ┃
┃    │                             │                                           │      ┃
┃ │  │                             ▼                                           │   │  ┃
┃    │                  ┌─────────────────────┐                                │      ┃
┃ │  │                  │    ChatStrategy     │                                │   │  ┃
┃    │                  │    (fully loaded)   │                                │       ┃
┃ │  │                  └──────────┬──────────┘                                │   │  ┃
┃    └─────────────────────────────┼───────────────────────────────────────────┘      ┃
┃ │                                │                                               │  ┃
┃ └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┼ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘  ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
                                   │
                                   │  engine.execute(&strategy, input, ...)
                                   │
                                   ▼
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  LAYER 1: EXECUTION ENGINE (pure, strategy-agnostic)                                 ┃
┃                                                                                      ┃
┃  ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐   ┃
┃                                                                                      ┃
┃  │  ╔═══════════════════════════════════════════════════════════════════════╗     │   ┃
┃     ║  ROUND 1                                                             ║        ┃
┃  │  ║                                                                      ║     │   ┃
┃     ║   ┌─────────────────────────────────────────────────────────────┐     ║        ┃
┃  │  ║   │                     SONNET LLM                              │     ║     │   ┃
┃     ║   │                                                             │     ║        ┃
┃  │  ║   │  system: "You are a helpful AI coding assistant...          │     ║     │   ┃
┃     ║   │           Focus on code quality and best practices..."      │     ║        ┃
┃  │  ║   │                                                             │     ║     │   ┃
┃     ║   │  messages:                                                  │     ║        ┃
┃  │  ║   │    [user]      "How do I center a div in CSS?"              │     ║     │   ┃
┃     ║   │    [assistant] "You can use flexbox..."                     │     ║        ┃
┃  │  ║   │    [user]      "Thanks! Now I have a different issue."      │     ║     │   ┃
┃     ║   │    [user]      "Help me debug this React component..."     │     ║        ┃
┃  │  ║   │                                                             │     ║     │   ┃
┃     ║   │  tools: [bash, read_file, edit_file, search_code]           │     ║        ┃
┃  │  ║   │  temp: 0.3  |  stream: true                                │     ║     │   ┃
┃     ║   └──────────────────────────┬──────────────────────────────────┘     ║        ┃
┃  │  ║                              │                                       ║     │   ┃
┃     ║                              ▼                                       ║        ┃
┃  │  ║          stop_reason: tool_use                                       ║     │   ┃
┃     ║          tool: search_code({ pattern: "useEffect|useState" })        ║        ┃
┃  │  ║                              │                                       ║     │   ┃
┃     ║                              ▼                                       ║        ┃
┃  │  ║          strategy.execute_tool("search_code", ...)                   ║     │   ┃
┃     ║          result: "Found 3 matches in Dashboard.tsx"                  ║        ┃
┃  │  ║                              │                                       ║     │   ┃
┃     ║                     append to messages                               ║        ┃
┃  │  ╚══════════════════════════════╪═══════════════════════════════════════╝     │   ┃
┃                                    │                                                 ┃
┃  │                                 ▼                                             │   ┃
┃     ╔═══════════════════════════════════════════════════════════════════════╗         ┃
┃  │  ║  ROUND 2                                                             ║     │   ┃
┃     ║                                                                      ║        ┃
┃  │  ║   ┌─────────────────────────────────────────────────────────────┐     ║     │   ┃
┃     ║   │                     SONNET LLM                              │     ║        ┃
┃  │  ║   │                                                             │     ║     │   ┃
┃     ║   │  messages: [history + round 1 tool call + tool result]      │     ║        ┃
┃  │  ║   │  (same system prompt, tools, temperature)                   │     ║     │   ┃
┃     ║   └──────────────────────────┬──────────────────────────────────┘     ║        ┃
┃  │  ║                              │                                       ║     │   ┃
┃     ║                              ▼                                       ║        ┃
┃  │  ║          stop_reason: tool_use                                       ║     │   ┃
┃     ║          tool: read_file({ path: "src/components/Dashboard.tsx" })   ║        ┃
┃  │  ║                              │                                       ║     │   ┃
┃     ║                              ▼                                       ║        ┃
┃  │  ║          strategy.execute_tool("read_file", ...)                     ║     │   ┃
┃     ║          result: "const Dashboard = () => { ... }"                   ║        ┃
┃  │  ║                              │                                       ║     │   ┃
┃     ║                     append to messages                               ║        ┃
┃  │  ╚══════════════════════════════╪═══════════════════════════════════════╝     │   ┃
┃                                    │                                                 ┃
┃  │                                 ▼                                             │   ┃
┃     ╔═══════════════════════════════════════════════════════════════════════╗         ┃
┃  │  ║  ROUND 3                                                             ║     │   ┃
┃     ║                                                                      ║        ┃
┃  │  ║   ┌─────────────────────────────────────────────────────────────┐     ║     │   ┃
┃     ║   │                     SONNET LLM                              │     ║        ┃
┃  │  ║   │                                                             │     ║     │   ┃
┃     ║   │  messages: [history + round 1 + round 2 tool results]       │     ║        ┃
┃  │  ║   └──────────────────────────┬──────────────────────────────────┘     ║     │   ┃
┃     ║                              │                                       ║        ┃
┃  │  ║                              ▼                                       ║     │   ┃
┃     ║          stop_reason: end_turn                                       ║        ┃
┃  │  ║                                                                      ║     │   ┃
┃     ║          "I found the issue. Your useEffect on line 14 creates       ║        ┃
┃  │  ║           a new object on every render, which triggers an            ║     │   ┃
┃     ║           infinite re-render loop. Here's the fix:                   ║        ┃
┃  │  ║                                                                      ║     │   ┃
┃     ║           // Before (broken):                                        ║        ┃
┃  │  ║           useEffect(() => { fetchData() }, [{ id: 1 }])             ║     │   ┃
┃     ║                                                                      ║        ┃
┃  │  ║           // After (fixed):                                          ║     │   ┃
┃     ║           const params = useMemo(() => ({ id: 1 }), [])             ║        ┃
┃  │  ║           useEffect(() => { fetchData() }, [params])"               ║     │   ┃
┃     ║                              │                                       ║        ┃
┃  │  ║                              ▼                                       ║     │   ┃
┃     ║          strategy.on_complete(response, usage)                       ║        ┃
┃  │  ║            ├── save assistant message to DB                          ║     │   ┃
┃     ║            ├── record 3,200 input / 850 output tokens               ║        ┃
┃  │  ║            ├── auto-name session (background)                        ║     │   ┃
┃     ║            └── trigger compaction if needed (background)             ║        ┃
┃  │  ║                                                                      ║     │   ┃
┃     ╚══════════════════════════════╪═══════════════════════════════════════╝         ┃
┃  │                                 │                                             │   ┃
┃  └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┼─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘   ┃
┃                                    │                                                 ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
                                     │
                                     │  ExecutionResult
                                     │
                                     ▼
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  OUTPUT                                                                              ┃
┃                                                                                      ┃
┃   ┌──────────────────────────────────────────────────────────────────────────────┐   ┃
┃   │  OrchestratedResult                                                         │   ┃
┃   │  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄                                                        │   ┃
┃   │  execution:                                                                  │   ┃
┃   │    content:      "I found the issue. Your useEffect on line 14..."          │   ┃
┃   │    input_tokens:  3,200                                                      │   ┃
┃   │    output_tokens: 850                                                        │   ┃
┃   │    rounds_used:   3                                                          │   ┃
┃   │                                                                              │   ┃
┃   │  selected_mode_id:  "m001-..."                                               │   ┃
┃   │  selected_mode_key: "coding"                                                 │   ┃
┃   └──────────────────────────────────────────────────────────────────────────────┘   ┃
┃                                                                                      ┃
┃   ┌──────────────────────────────────────────────────────────────────────────────┐   ┃
┃   │  INSERT INTO agent_executions                                                │   ┃
┃   │    agent_id:                'a1b2c3d4-...'                                   │   ┃
┃   │    selected_router_mode_id: 'm001-...'  (coding)                             │   ┃
┃   │    input:                   'Help me debug this React component...'          │   ┃
┃   │    output:                  'I found the issue. Your useEffect...'           │   ┃
┃   └──────────────────────────────────────────────────────────────────────────────┘   ┃
┃                                                                                      ┃
┃   ┌──────────────────────────────────────────────────────────────────────────────┐   ┃
┃   │  SSE Stream --> WebSocket --> User's browser                                 │   ┃
┃   └──────────────────────────────────────────────────────────────────────────────┘   ┃
┃                                                                                      ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛


 DB READS SUMMARY                          LLM CALLS SUMMARY
 ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄                        ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄

 ┌────────────────────────────┐            ┌────────────────────────────┐
 │ 1. agents (agent config)   │            │ 1. Haiku   (classification)│
 │ 2. tool_routers (router)   │            │    ~200 in / ~5 out tokens │
 │ 3. tool_router_modes (x3)  │            │                            │
 │ 4. tool_router_mode_tools  │            │ 2. Sonnet  (round 1)       │
 │ 5. tools (x4 definitions)  │            │ 3. Sonnet  (round 2)       │
 │ 6. session_messages (hist) │            │ 4. Sonnet  (round 3)       │
 └────────────────────────────┘            │    ~3,200 in / ~850 out    │
                                           └────────────────────────────┘
 6 queries                                 4 LLM calls (1 cheap + 3 main)


 TOKEN SAVINGS
 ┄┄┄┄┄┄┄┄┄┄┄┄┄

 Before (50 tools):     29,160 input tokens ████████████████████████████████████████
 After  ( 4 tools):      7,405 input tokens ██████████▏
                                              ▲
                                              │
                                         75% reduction
```
