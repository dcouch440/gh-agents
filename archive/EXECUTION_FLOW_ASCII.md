# Router Modes Execution Flow (ASCII)

```
 USER MESSAGE: "Help me debug this React component that keeps re-rendering"
 =============================================================================

                                    |
                                    v
 =====================================================================
 |                        AGENT ORCHESTRATOR                          |
 |                                                                    |
 |  [1] LOAD AGENT                                                    |
 |  +-----------------------------------------------------------------+
 |  | SELECT * FROM agents WHERE id = 'a1b2c3d4-...'                  |
 |  |                                                                  |
 |  |  AgentRow:                                                       |
 |  |    name:          "CodeBot"                                      |
 |  |    system_prompt: "You are a helpful AI coding assistant.        |
 |  |                    You help users write, debug, and improve      |
 |  |                    code across all languages and frameworks."    |
 |  |    model_id:      "claude-sonnet-4-20250514"                     |
 |  |    temperature:   0.7                                            |
 |  |    router_id:     "r1r2r3r4-..."  <-- HAS ROUTER                |
 |  +-----------------------------------------------------------------+
 |                                    |
 |                       router_id is Some(...)
 |                       so we proceed to routing
 |                                    |
 |                                    v
 |  [2] LOAD ROUTER                                                   |
 |  +-----------------------------------------------------------------+
 |  | SELECT * FROM tool_routers WHERE id = 'r1r2r3r4-...'            |
 |  |                                                                  |
 |  |  ToolRouterRow:                                                  |
 |  |    name:          "CodeBot Task Router"                          |
 |  |    system_prompt: "You are a conversation classifier. Given      |
 |  |                    the user's message and conversation history,  |
 |  |                    select the most appropriate mode. Respond     |
 |  |                    with ONLY the mode key, nothing else."        |
 |  |    model_id:      "claude-haiku-4-20250414"                      |
 |  |    level:         1                                              |
 |  +-----------------------------------------------------------------+
 |                                    |
 |                                    v
 |  [3] LOAD MODES                                                    |
 |  +-----------------------------------------------------------------+
 |  | SELECT * FROM tool_router_modes                                  |
 |  | WHERE router_id = 'r1r2r3r4-...' ORDER BY display_order         |
 |  |                                                                  |
 |  |  +-----------------------------------------------------------+  |
 |  |  | mode_key: "coding"       | mode_key: "research"           |  |
 |  |  | description: "For        | description: "For info         |  |
 |  |  |   programming, debugging, |   gathering, documentation    |  |
 |  |  |   code review, and        |   lookup, and analysis"       |  |
 |  |  |   development tasks"      |                               |  |
 |  |  | system_prompt: "Focus on  | system_prompt: "Focus on      |  |
 |  |  |   code quality and best   |   finding accurate info.      |  |
 |  |  |   practices. Be precise   |   Cite sources. Summarize     |  |
 |  |  |   and technical..."       |   findings clearly."          |  |
 |  |  | temperature: 0.3          | temperature: 0.5              |  |
 |  |  | append_system_prompt: T   | append_system_prompt: T       |  |
 |  |  | append_tools: F (REPLACE) | append_tools: F (REPLACE)     |  |
 |  |  +---------------------------+-------------------------------+  |
 |  |  | mode_key: "chat"                                          |  |
 |  |  | description: "For general conversation, explanations"     |  |
 |  |  | system_prompt: "Be conversational and friendly..."        |  |
 |  |  | temperature: 0.9                                          |  |
 |  |  | append_system_prompt: F (REPLACE)                         |  |
 |  |  | append_tools: F (REPLACE)                                 |  |
 |  |  +-----------------------------------------------------------+  |
 |  +-----------------------------------------------------------------+
 |                                    |
 |                                    v
 |  [4] CLASSIFY (Router LLM Call)                                    |
 |  +-----------------------------------------------------------------+
 |  |                                                                  |
 |  |  RouterStrategy --> ExecutionEngine --> Haiku LLM                |
 |  |                                                                  |
 |  |  SYSTEM PROMPT:                                                  |
 |  |  +------------------------------------------------------------+ |
 |  |  | "You are a conversation classifier. Given the user's        | |
 |  |  |  message and conversation history, select the most          | |
 |  |  |  appropriate mode. Respond with ONLY the mode key,          | |
 |  |  |  nothing else."                                             | |
 |  |  +------------------------------------------------------------+ |
 |  |                                                                  |
 |  |  USER MESSAGE:                                                   |
 |  |  +------------------------------------------------------------+ |
 |  |  | ## Conversation History (last 10 messages):                 | |
 |  |  | user: How do I center a div in CSS?                         | |
 |  |  | assistant: You can use flexbox: display: flex; ...          | |
 |  |  | user: Thanks! Now I have a different issue.                 | |
 |  |  |                                                             | |
 |  |  | ## Current User Input:                                      | |
 |  |  | Help me debug this React component that keeps re-rendering  | |
 |  |  |                                                             | |
 |  |  | ## Available Modes:                                         | |
 |  |  | - coding: For programming, debugging, code review...        | |
 |  |  | - research: For info gathering, documentation lookup...      | |
 |  |  | - chat: For general conversation, explanations...            | |
 |  |  |                                                             | |
 |  |  | Based on the conversation context and current input,         | |
 |  |  | output ONLY the mode key.                                   | |
 |  |  +------------------------------------------------------------+ |
 |  |                                                                  |
 |  |  MODEL: claude-haiku-4-20250414                                  |
 |  |  TEMP:  0.0  (deterministic)                                     |
 |  |  TOOLS: []   (none)                                              |
 |  |                                                                  |
 |  |  RESPONSE: "coding"                                              |
 |  |            ~~~~~~~~                                              |
 |  |  COST: ~200 input tokens, ~5 output tokens                      |
 |  |                                                                  |
 |  +-----------------------------------------------------------------+
 |                                    |
 |                         selected mode = "coding"
 |                                    |
 |                                    v
 |  [5] LOAD MODE TOOLS                                               |
 |  +-----------------------------------------------------------------+
 |  | SELECT t.* FROM tools t                                          |
 |  | INNER JOIN tool_router_mode_tools trmt ON t.id = trmt.tool_id   |
 |  | WHERE trmt.mode_id = 'm001-...'                                  |
 |  |                                                                  |
 |  |  +------------+  +------------+  +------------+  +------------+ |
 |  |  | bash       |  | read_file  |  | edit_file  |  | search_code| |
 |  |  | "Execute   |  | "Read the  |  | "Edit a    |  | "Search    | |
 |  |  |  bash      |  |  contents  |  |  file with |  |  for code  | |
 |  |  |  commands" |  |  of a file"|  |  search &  |  |  patterns" | |
 |  |  |            |  |            |  |  replace"  |  |            | |
 |  |  +------------+  +------------+  +------------+  +------------+ |
 |  |                                                                  |
 |  |  4 tools loaded (agent normally has 50)                          |
 |  +-----------------------------------------------------------------+
 |                                    |
 |                                    v
 |  [6] RESOLVE FINAL CONFIG                                          |
 |  +-----------------------------------------------------------------+
 |  |                                                                  |
 |  |  SYSTEM PROMPT (append_to_agent_system_prompt = true):          |
 |  |  +------------------------------------------------------------+ |
 |  |  | "You are a helpful AI coding assistant. You help users      | |
 |  |  |  write, debug, and improve code across all languages        | |
 |  |  |  and frameworks."                           <-- from agent  | |
 |  |  |                                                             | |
 |  |  | "Focus on code quality and best practices. Be precise       | |
 |  |  |  and technical. Show code examples. Use debugging tools     | |
 |  |  |  when appropriate. Think step-by-step through problems."    | |
 |  |  |                                             <-- from mode   | |
 |  |  +------------------------------------------------------------+ |
 |  |                                                                  |
 |  |  TOOLS (append_to_agent_tools = false):                         |
 |  |  +------------------------------------------------------------+ |
 |  |  | Agent tools:  [read_file, write_file, bash, git, edit_file, | |
 |  |  |                search_code, web_search, ... 40 more]        | |
 |  |  |                                              DISCARDED  X   | |
 |  |  |                                                             | |
 |  |  | Mode tools:   [bash, read_file, edit_file, search_code]     | |
 |  |  |                                              USED  <---     | |
 |  |  +------------------------------------------------------------+ |
 |  |                                                                  |
 |  |  TEMPERATURE:  0.3  (from mode, overrides agent's 0.7)          |
 |  |  MODEL:        claude-sonnet-4-20250514  (from agent)           |
 |  |  MAX TOKENS:   8000  (from mode)                                |
 |  |                                                                  |
 |  +-----------------------------------------------------------------+
 |                                    |
 |               Build ChatStrategy with this config
 |                                    |
 =====================================================================
                                    |
                                    v
 =====================================================================
 |                       EXECUTION ENGINE                             |
 |                   (unchanged, strategy-agnostic)                   |
 |                                                                    |
 |  strategy.build_messages("Help me debug this React component...")  |
 |      |                                                             |
 |      v                                                             |
 |  Load session history from DB:                                     |
 |  +---------------------------------------------------------------+ |
 |  | [user]      "How do I center a div in CSS?"                    | |
 |  | [assistant] "You can use flexbox: display: flex; ..."          | |
 |  | [user]      "Thanks! Now I have a different issue."            | |
 |  | [user]      "Help me debug this React component that keeps     | |
 |  |              re-rendering"                                     | |
 |  +---------------------------------------------------------------+ |
 |                                    |
 |                                    v
 |  ROUND 1                                                           |
 |  +---------------------------------------------------------------+ |
 |  | LLM REQUEST:                                                   | |
 |  |   model:       claude-sonnet-4-20250514                        | |
 |  |   system:      [agent prompt + mode prompt]                    | |
 |  |   messages:    [4 history messages]                             | |
 |  |   tools:       [bash, read_file, edit_file, search_code]       | |
 |  |   temperature: 0.3                                              | |
 |  |   stream:      true                                             | |
 |  |                                                                 | |
 |  | LLM RESPONSE:                                                   | |
 |  |   stop_reason: tool_use                                         | |
 |  |   text: "Let me look at your React component to understand      | |
 |  |          the re-rendering issue."                                | |
 |  |   tool_use:                                                      | |
 |  |     name:  search_code                                           | |
 |  |     input: { pattern: "useEffect|useState", path: "src/" }      | |
 |  |                                                                 | |
 |  | TOOL EXECUTION:                                                  | |
 |  |   strategy.execute_tool("search_code", {...})                    | |
 |  |   result: "Found 3 matches in src/components/Dashboard.tsx"      | |
 |  |                                                                 | |
 |  | --> append assistant msg + tool result to messages, loop         | |
 |  +---------------------------------------------------------------+ |
 |                                    |
 |                                    v
 |  ROUND 2                                                           |
 |  +---------------------------------------------------------------+ |
 |  | LLM REQUEST:                                                   | |
 |  |   messages: [4 history + assistant + tool_result from round 1] | |
 |  |   (same system, tools, temp)                                   | |
 |  |                                                                 | |
 |  | LLM RESPONSE:                                                   | |
 |  |   stop_reason: tool_use                                         | |
 |  |   text: "I can see the issue. Let me read the Dashboard."       | |
 |  |   tool_use:                                                      | |
 |  |     name:  read_file                                             | |
 |  |     input: { path: "src/components/Dashboard.tsx" }              | |
 |  |                                                                 | |
 |  | TOOL EXECUTION:                                                  | |
 |  |   strategy.execute_tool("read_file", {...})                      | |
 |  |   result: "const Dashboard = () => { const [data, setData]..."   | |
 |  |                                                                 | |
 |  | --> append assistant msg + tool result to messages, loop         | |
 |  +---------------------------------------------------------------+ |
 |                                    |
 |                                    v
 |  ROUND 3                                                           |
 |  +---------------------------------------------------------------+ |
 |  | LLM REQUEST:                                                   | |
 |  |   messages: [history + round 1 + round 2 tool results]        | |
 |  |                                                                 | |
 |  | LLM RESPONSE:                                                   | |
 |  |   stop_reason: end_turn                                         | |
 |  |   text: "I found the issue. Your useEffect on line 14 creates  | |
 |  |          a new object on every render, which triggers an        | |
 |  |          infinite re-render loop. Here's the fix:               | |
 |  |                                                                 | |
 |  |          // Before (broken):                                    | |
 |  |          useEffect(() => { fetchData() }, [{ id: 1 }])         | |
 |  |                                                                 | |
 |  |          // After (fixed):                                      | |
 |  |          const params = useMemo(() => ({ id: 1 }), [])         | |
 |  |          useEffect(() => { fetchData() }, [params])"           | |
 |  |                                                                 | |
 |  |   usage: { input: 3200, output: 850 }                          | |
 |  |                                                                 | |
 |  | strategy.on_complete(response, usage)                            | |
 |  |   --> save assistant message to DB                               | |
 |  |   --> record tokens to ledger                                    | |
 |  |   --> auto-name session (background)                             | |
 |  |   --> trigger compaction if needed (background)                  | |
 |  +---------------------------------------------------------------+ |
 |                                                                    |
 =====================================================================
                                    |
                                    v
 =====================================================================
 |                          SAVE & RETURN                             |
 |                                                                    |
 |  INSERT INTO agent_executions (                                    |
 |      agent_id:                'a1b2c3d4-...',                      |
 |      selected_router_mode_id: 'm001-...',  <-- tracks "coding"    |
 |      input:                   'Help me debug this React...',       |
 |      output:                  'I found the issue. Your useEffect.' |
 |  )                                                                 |
 |                                                                    |
 |  Return to caller:                                                 |
 |  +---------------------------------------------------------------+ |
 |  | OrchestratedResult {                                           | |
 |  |   execution: ExecutionResult {                                 | |
 |  |     content:      "I found the issue. Your useEffect...",      | |
 |  |     input_tokens:  3200,                                       | |
 |  |     output_tokens: 850,                                        | |
 |  |     rounds_used:   3,                                          | |
 |  |   },                                                           | |
 |  |   selected_mode_id:  Some("m001-..."),                         | |
 |  |   selected_mode_key: Some("coding"),                           | |
 |  | }                                                              | |
 |  +---------------------------------------------------------------+ |
 |                                                                    |
 |  Streamed to user via SSE websocket                                |
 |                                                                    |
 =====================================================================


 TOKEN COST COMPARISON
 =====================

 BEFORE (no routing):                AFTER (with routing):
 +----------------------------+      +----------------------------+
 | System prompt:    200 tok  |      | ROUTER CALL (Haiku):       |
 | Tools (50):     8,000 tok  |      |   Input:        200 tok    |
 | History:        1,500 tok  |      |   Output:         5 tok    |
 | User message:      20 tok  |      |                            |
 | ─────────────────────────  |      | MAIN CALL (Sonnet):        |
 | Per round:      9,720 tok  |      |   System prompt: 280 tok   |
 | x 3 rounds                 |      |   Tools (4):     600 tok   |
 | ─────────────────────────  |      |   History:     1,500 tok   |
 | TOTAL:         29,160 tok  |      |   User message:   20 tok   |
 |                            |      |   ────────────────────────  |
 |                            |      |   Per round:   2,400 tok   |
 |                            |      |   x 3 rounds               |
 |                            |      |   ────────────────────────  |
 |                            |      |   TOTAL:       7,405 tok   |
 +----------------------------+      +----------------------------+

                    75% REDUCTION IN INPUT TOKENS
```
