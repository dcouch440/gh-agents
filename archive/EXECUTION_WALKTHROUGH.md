# Execution Walkthrough: Router Modes System

End-to-end trace of a chat message with the new router modes system.
Every database record, every LLM call, every data transformation.

---

## Scenario

User sends: **"Help me debug this React component that keeps re-rendering"**

The agent "CodeBot" has a router attached with 3 modes: `coding`, `research`, `chat`.

---

## Step 1: Application Layer (chat_consumer)

The WebSocket handler receives the message and calls the orchestrator.

```
orchestrator.execute_agent(
    agent_id:   "a1b2c3d4-...",
    input:      "Help me debug this React component that keeps re-rendering",
    history:    [last 10 messages],
    sink:       SseSink,
    recorder:   ExecutionRecorder,
    cancel:     CancellationToken,
)
```

---

## Step 2: Load Agent from DB

```sql
SELECT * FROM agents WHERE id = 'a1b2c3d4-...';
```

**Returns `AgentRow`:**

```rust
AgentRow {
    id:                "a1b2c3d4-...",
    name:              "CodeBot",
    system_prompt:     "You are a helpful AI coding assistant. You help users write,
                        debug, and improve code across all languages and frameworks.",
    model_provider:    "anthropic",
    model_id:          "claude-sonnet-4-20250514",
    model_max_tokens:  4096,
    model_temperature: 0.7,
    router_id:         Some("r1r2r3r4-..."),   // <-- HAS A ROUTER
    ...
}
```

**Decision point:** `agent.router_id` is `Some(...)` --> proceed to routing.
If it were `None`, skip straight to Step 7 using agent defaults.

---

## Step 3: Load Router from DB

```sql
SELECT * FROM tool_routers WHERE id = 'r1r2r3r4-...';
```

**Returns `ToolRouterRow`:**

```rust
ToolRouterRow {
    id:               "r1r2r3r4-...",
    name:             "CodeBot Task Router",
    system_prompt:    "You are a conversation classifier. Given the user's message
                       and conversation history, select the most appropriate mode.
                       Respond with ONLY the mode key, nothing else.",
    model_id:         "claude-haiku-4-20250414",
    is_active:        true,
    level:            1,
    parent_router_id: None,
    ...
}
```

---

## Step 4: Load Router Modes from DB

```sql
SELECT * FROM tool_router_modes WHERE router_id = 'r1r2r3r4-...' ORDER BY display_order;
```

**Returns `Vec<ToolRouterModeRow>`:**

```rust
[
    ToolRouterModeRow {
        id:                           "m001-...",
        router_id:                    "r1r2r3r4-...",
        mode_key:                     "coding",
        display_name:                 "Coding Mode",
        description:                  "For programming, debugging, code review, and development tasks",
        system_prompt:                "Focus on code quality and best practices. Be precise and
                                       technical. Show code examples. Use debugging tools when
                                       appropriate. Think step-by-step through problems.",
        temperature:                  0.3,
        max_tokens:                   8000,
        append_to_agent_system_prompt: true,    // APPEND to agent prompt
        append_to_agent_tools:         false,   // REPLACE agent tools
        display_order:                 0,
        ...
    },
    ToolRouterModeRow {
        id:                           "m002-...",
        mode_key:                     "research",
        description:                  "For information gathering, documentation lookup, and analysis",
        system_prompt:                "Focus on finding accurate information. Cite sources.
                                       Summarize findings clearly.",
        temperature:                  0.5,
        max_tokens:                   4096,
        append_to_agent_system_prompt: true,
        append_to_agent_tools:         false,
        display_order:                 1,
        ...
    },
    ToolRouterModeRow {
        id:                           "m003-...",
        mode_key:                     "chat",
        description:                  "For general conversation, explanations, and non-technical discussion",
        system_prompt:                "Be conversational and friendly. Explain concepts clearly
                                       without unnecessary jargon.",
        temperature:                  0.9,
        max_tokens:                   2048,
        append_to_agent_system_prompt: false,   // REPLACE agent prompt entirely
        append_to_agent_tools:         false,   // REPLACE agent tools
        display_order:                 2,
        ...
    },
]
```

---

## Step 5: Classify — Router LLM Call

The orchestrator builds a classification prompt and calls the router LLM
using a `RouterStrategy` through the `ExecutionEngine`.

### 5a. Build classification prompt

```
## Conversation History (last 10 messages):
user: How do I center a div in CSS?
assistant: You can use flexbox: display: flex; justify-content: center; align-items: center;
user: Thanks! Now I have a different issue.
user: Help me debug this React component that keeps re-rendering

## Current User Input:
Help me debug this React component that keeps re-rendering

## Available Modes:
- coding: For programming, debugging, code review, and development tasks
- research: For information gathering, documentation lookup, and analysis
- chat: For general conversation, explanations, and non-technical discussion

Based on the conversation context and current input, output ONLY the mode key.
```

### 5b. Create RouterStrategy

```rust
RouterStrategy {
    config: RouterConfig {
        system_prompt: "You are a conversation classifier. Given the user's message
                        and conversation history, select the most appropriate mode.
                        Respond with ONLY the mode key, nothing else.",
        model_id:      "claude-haiku-4-20250414",
        state:         Some(app_state),
        user_id:       Some(user_id),
    }
}
```

### 5c. Engine executes RouterStrategy

```
engine.execute(&router_strategy, classification_prompt, &NullSink, &recorder, None)
```

**What the Router LLM actually receives:**

```json
{
    "model": "claude-haiku-4-20250414",
    "system": "You are a conversation classifier. Given the user's message
               and conversation history, select the most appropriate mode.
               Respond with ONLY the mode key, nothing else.",
    "messages": [
        {
            "role": "user",
            "content": "## Conversation History (last 10 messages):\nuser: How do I center...\n\n## Current User Input:\nHelp me debug this React component that keeps re-rendering\n\n## Available Modes:\n- coding: For programming...\n- research: For information...\n- chat: For general conversation...\n\nBased on the conversation context and current input, output ONLY the mode key."
        }
    ],
    "max_tokens": 4096,
    "temperature": 0.0,
    "tools": []
}
```

**Router LLM responds:**

```
coding
```

### 5d. Parse response

```rust
mode_key = "coding"  // parsed from LLM response
```

If the response were invalid (e.g. "I think coding would be best"), fallback
to first mode by `display_order` → still `"coding"`.

---

## Step 6: Load Mode Tools from DB

```sql
SELECT t.*
FROM tools t
INNER JOIN tool_router_mode_tools trmt ON t.id = trmt.tool_id
WHERE trmt.mode_id = 'm001-...';
```

**Returns `Vec<ToolRow>`:**

```rust
[
    ToolRow {
        id:           "t001-...",
        name:         "bash",
        display_name: "Bash",
        description:  "Execute bash commands on the system",
        parameters:   { "type": "object", "properties": { "command": { "type": "string" } }, "required": ["command"] },
        ...
    },
    ToolRow {
        id:           "t002-...",
        name:         "read_file",
        display_name: "Read File",
        description:  "Read the contents of a file",
        parameters:   { "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] },
        ...
    },
    ToolRow {
        id:           "t003-...",
        name:         "edit_file",
        display_name: "Edit File",
        description:  "Edit a file with search and replace",
        parameters:   { ... },
        ...
    },
    ToolRow {
        id:           "t004-...",
        name:         "search_code",
        display_name: "Search Code",
        description:  "Search for patterns in the codebase",
        parameters:   { ... },
        ...
    },
]
```

**4 tools, not 50.**

---

## Step 6b: Resolve System Prompt (append vs replace)

The selected mode has `append_to_agent_system_prompt: true`, so:

```
Agent system_prompt:
    "You are a helpful AI coding assistant. You help users write,
     debug, and improve code across all languages and frameworks."

Mode system_prompt:
    "Focus on code quality and best practices. Be precise and
     technical. Show code examples. Use debugging tools when
     appropriate. Think step-by-step through problems."

─── APPEND ───

Final system_prompt:
    "You are a helpful AI coding assistant. You help users write,
     debug, and improve code across all languages and frameworks.

     Focus on code quality and best practices. Be precise and
     technical. Show code examples. Use debugging tools when
     appropriate. Think step-by-step through problems."
```

## Step 6c: Resolve Tools (append vs replace)

The selected mode has `append_to_agent_tools: false`, so:

```
Agent tools (from agent_tools table):
    [read_file, write_file, bash, git, edit_file, search_code,
     web_search, web_fetch, create_pr, list_issues, ... 40 more]

Mode tools (from tool_router_mode_tools table):
    [bash, read_file, edit_file, search_code]

─── REPLACE ───

Final tools:
    [bash, read_file, edit_file, search_code]
```

Agent's 50 tools are thrown away. Only the 4 mode tools are sent.

---

## Step 7: Build ChatStrategy

```rust
let chat_config = ChatConfig {
    system_prompt:  final_system_prompt,    // from step 6b
    tool_names:     ["bash", "read_file", "edit_file", "search_code"],  // from step 6c
    model_id:       "claude-sonnet-4-20250514",  // from agent
    temperature:    0.3,                    // from MODE (overrides agent's 0.7)
    max_history:    50,
    max_rounds:     10,
    context_budget: 480_000,
};

let strategy = ChatStrategy::new(chat_config, state, user_id, session_id, message_id);
```

---

## Step 8: Engine Executes ChatStrategy

```
engine.execute(&strategy, "Help me debug this React component...", &sse_sink, &recorder, cancel)
```

### 8a. strategy.build_messages() loads session history

```sql
SELECT * FROM session_messages WHERE session_id = '...' ORDER BY created_at LIMIT 50;
```

**Messages built:**

```rust
[
    Message::user("How do I center a div in CSS?"),
    Message::assistant("You can use flexbox: display: flex; ..."),
    Message::user("Thanks! Now I have a different issue."),
    Message::user("Help me debug this React component that keeps re-rendering"),
]
```

### 8b. What the Main LLM actually receives (Round 1)

```json
{
    "model": "claude-sonnet-4-20250514",
    "system": "You are a helpful AI coding assistant. You help users write, debug, and improve code across all languages and frameworks.\n\nFocus on code quality and best practices. Be precise and technical. Show code examples. Use debugging tools when appropriate. Think step-by-step through problems.",
    "messages": [
        { "role": "user",      "content": "How do I center a div in CSS?" },
        { "role": "assistant", "content": "You can use flexbox: display: flex; ..." },
        { "role": "user",      "content": "Thanks! Now I have a different issue." },
        { "role": "user",      "content": "Help me debug this React component that keeps re-rendering" }
    ],
    "max_tokens": 4096,
    "temperature": 0.3,
    "stream": true,
    "tools": [
        {
            "name": "bash",
            "description": "Execute bash commands on the system",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The bash command to execute" }
                },
                "required": ["command"]
            }
        },
        {
            "name": "read_file",
            "description": "Read the contents of a file",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the file" }
                },
                "required": ["path"]
            }
        },
        {
            "name": "edit_file",
            "description": "Edit a file with search and replace",
            "input_schema": { "..." }
        },
        {
            "name": "search_code",
            "description": "Search for patterns in the codebase",
            "input_schema": { "..." }
        }
    ]
}
```

**Key differences from the old system:**
- `temperature: 0.3` (from mode, not agent's default 0.7)
- `tools`: 4 items (not 50)
- `system`: agent prompt + mode prompt appended

### 8c. LLM responds with tool use (Round 1)

```json
{
    "stop_reason": "tool_use",
    "content": [
        {
            "type": "text",
            "text": "Let me look at your React component to understand the re-rendering issue."
        },
        {
            "type": "tool_use",
            "id": "toolu_01ABC",
            "name": "search_code",
            "input": { "pattern": "useEffect|useState|useMemo", "path": "src/components" }
        }
    ]
}
```

**Engine calls:** `strategy.execute_tool("search_code", { pattern: "...", path: "..." })`
**Tool returns:** `"Found 3 matches in src/components/Dashboard.tsx ..."`

Engine appends assistant message + tool result to messages, loops.

### 8d. LLM responds with tool use (Round 2)

```json
{
    "stop_reason": "tool_use",
    "content": [
        {
            "type": "text",
            "text": "I can see the issue. Let me read the Dashboard component."
        },
        {
            "type": "tool_use",
            "id": "toolu_02DEF",
            "name": "read_file",
            "input": { "path": "src/components/Dashboard.tsx" }
        }
    ]
}
```

**Engine calls:** `strategy.execute_tool("read_file", { path: "..." })`
**Tool returns:** `"const Dashboard = () => { const [data, setData] = useState(...)... }"`

### 8e. LLM responds with final answer (Round 3)

```json
{
    "stop_reason": "end_turn",
    "content": [
        {
            "type": "text",
            "text": "I found the issue. Your `useEffect` on line 14 creates a new object on every render..."
        }
    ],
    "usage": {
        "input_tokens": 3200,
        "output_tokens": 850
    }
}
```

**Engine calls:** `strategy.on_complete(response, usage)` which saves the message,
records token usage, auto-names the session, etc.

---

## Step 9: Record Execution

```sql
INSERT INTO agent_executions (
    id,
    agent_id,
    selected_router_mode_id,   -- "m001-..." (coding mode)
    input,
    output,
    ...
) VALUES (...);
```

**This tracks which mode was used, enabling analytics:**
- "80% of messages route to coding mode"
- "research mode has 2x higher token usage"
- "chat mode has the highest user satisfaction"

---

## Complete Flow Diagram

```
USER MESSAGE
"Help me debug this React component that keeps re-rendering"
│
▼
┌─────────────────────────────────────────────────────────────────┐
│ ORCHESTRATOR                                                     │
│                                                                  │
│  ┌─── DB READ ───────────────────────────────────────────────┐  │
│  │ agents        → AgentRow (system_prompt, model, router_id)│  │
│  │ tool_routers  → ToolRouterRow (classification prompt)     │  │
│  │ router_modes  → 3x ToolRouterModeRow (coding/research/   │  │
│  │                                        chat)              │  │
│  └───────────────────────────────────────────────────────────┘  │
│                         │                                        │
│                         ▼                                        │
│  ┌─── ROUTER LLM CALL (Haiku, temp=0.0) ────────────────────┐  │
│  │                                                            │  │
│  │  System: "You are a conversation classifier..."           │  │
│  │                                                            │  │
│  │  User:   "## History:\n...\n## Input:\nHelp me debug...\n │  │
│  │           ## Modes:\n- coding: ...\n- research: ...\n     │  │
│  │           - chat: ...\nOutput ONLY the mode key."         │  │
│  │                                                            │  │
│  │  Response: "coding"                                        │  │
│  │                                                            │  │
│  │  Cost: ~200 input tokens, ~5 output tokens ($0.00005)     │  │
│  └────────────────────────────────────────────────────────────┘  │
│                         │                                        │
│                         ▼                                        │
│  ┌─── DB READ ───────────────────────────────────────────────┐  │
│  │ tool_router_mode_tools → 4x ToolRow (bash, read_file,    │  │
│  │                                       edit_file,          │  │
│  │                                       search_code)        │  │
│  └───────────────────────────────────────────────────────────┘  │
│                         │                                        │
│                         ▼                                        │
│  ┌─── BUILD CONFIG ──────────────────────────────────────────┐  │
│  │                                                            │  │
│  │  System prompt: agent.prompt + "\n\n" + mode.prompt       │  │
│  │  Temperature:   0.3  (from mode)                          │  │
│  │  Tools:         [bash, read_file, edit_file, search_code] │  │
│  │  Model:         claude-sonnet-4-20250514 (from agent)     │  │
│  │                                                            │  │
│  └────────────────────────────────────────────────────────────┘  │
│                         │                                        │
│                         │  ChatStrategy                          │
│                         ▼                                        │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│ EXECUTION ENGINE (unchanged, strategy-agnostic)                  │
│                                                                  │
│  Round 1:                                                        │
│  ├─ strategy.build_messages() → [history + current message]     │
│  ├─ LLM request (Sonnet, temp=0.3, 4 tools, streaming)         │
│  ├─ Response: tool_use → search_code("useEffect|useState")     │
│  ├─ strategy.execute_tool("search_code", ...) → results        │
│  └─ Append to messages, loop                                    │
│                                                                  │
│  Round 2:                                                        │
│  ├─ LLM request (messages now include tool result)              │
│  ├─ Response: tool_use → read_file("Dashboard.tsx")             │
│  ├─ strategy.execute_tool("read_file", ...) → file contents    │
│  └─ Append to messages, loop                                    │
│                                                                  │
│  Round 3:                                                        │
│  ├─ LLM request (messages now include both tool results)        │
│  ├─ Response: end_turn → "I found the issue. Your useEffect..." │
│  ├─ strategy.on_complete() → save message, record tokens        │
│  └─ Return ExecutionResult                                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│ RESULT                                                           │
│                                                                  │
│  ExecutionResult {                                               │
│      content:      "I found the issue. Your useEffect on ...",  │
│      input_tokens:  3200,                                       │
│      output_tokens: 850,                                        │
│      rounds_used:   3,                                          │
│  }                                                               │
│                                                                  │
│  OrchestratedResult {                                            │
│      execution:        ExecutionResult { ... },                  │
│      selected_mode_id: Some("m001-..."),   // coding            │
│      selected_mode_key: Some("coding"),                          │
│  }                                                               │
│                                                                  │
│  Saved to agent_executions with selected_router_mode_id          │
│  Streamed to user via SSE                                        │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Token Cost Comparison

### Before (no routing)

```
System prompt:     ~200 tokens   (agent prompt only)
Tools:             ~8,000 tokens (50 tool definitions)
History:           ~1,500 tokens
User message:      ~20 tokens
                   ──────────
Input per round:   ~9,720 tokens
x 3 rounds:        29,160 input tokens
```

### After (with routing)

```
Router call:
  Input:           ~200 tokens
  Output:          ~5 tokens

Main LLM call:
  System prompt:   ~280 tokens   (agent + mode prompt)
  Tools:           ~600 tokens   (4 tool definitions)
  History:         ~1,500 tokens
  User message:    ~20 tokens
                   ──────────
  Input per round: ~2,400 tokens
  x 3 rounds:      7,200 input tokens

Total:             7,405 tokens (vs 29,160 = 75% reduction)
```
