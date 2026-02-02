# Tool Router & Context Window Design

The end-game architecture for async tool routing, agent-to-agent communication, collective memory, and generative pipelines.

---

## Part 1: The Router

### Core Principle

The agent is a pure function. Context in, response out. Everything else — routing, async execution, chaining, ranking, expiry — lives in the router and context store.

There is no message history fed to the LLM. There is a **context window** that gets assembled fresh before every LLM call.

### The Agent's View

The agent sees one tool:

```json
{
  "name": "request",
  "description": "Send a request to be fulfilled. Describe what you need in plain language.",
  "parameters": {
    "intent": "What you need done, in natural language",
    "priority": "low | normal | high",
    "callback_hint": "Optional: what you plan to do with the result"
  }
}
```

That's it. The agent never sees `search_web`, `query_db`, `run_code`, etc.

### The Flow

```
Agent calls request("find Portland diners")
         │
         ▼
    Router LLM (has ALL tool specs + routing rules)
         │
         ├──→ Picks tool(s): search_web
         ├──→ Decides: async (network latency)
         ├──→ Generates passdown: "Looking into Portland's dining scene..."
         │
         ├──→ Returns to agent immediately:
         │    { async: true, passdown: "Looking into that..." }
         │
         └──→ Background: executes search_web(...)
                   │
                   ▼
              Result lands in context store
                   │
                   ▼
              Next LLM call picks it up automatically
```

### The Passdown Rule

**If you're making them wait, you owe them words.**

Every async tool call requires a `passdown` — something for the agent to hand the user while work runs. This is enforced at the schema level:

```json
{
  "tool": "search_web",
  "tool_args": { "query": "Portland diners" },
  "async": true,
  "passdown": "Let me look into Portland's dining scene — any cuisine you're partial to?"
}
```

The router cannot return `async: true` without a `passdown`. Sync calls don't need one because the result is immediate.

### Context Window Assembly

The context window is a living document rebuilt before every LLM call:

```
┌──────────────────────────────────────────────────────┐
│                   CONTEXT WINDOW                      │
│                                                      │
│  ┌─ agent identity ───────────────────────────────┐  │
│  │ system prompt + mode + available tools          │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─ conversation ──────────────────────────────────┐ │
│  │ user: "Find me diners in Portland"              │ │
│  │ assistant: "On it — any cuisine preference?"    │ │
│  │ user: "Thai if possible"                        │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─ resolved results (priority ranked) ────────────┐ │
│  │ [search_web] 5 diners found, 2 Thai             │ │
│  │ [get_reviews] Pok Pok: 4.7★, Hat Yai: 4.5★     │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─ in-flight (agent knows these are pending) ─────┐ │
│  │ [check_reservations] ⏳ waiting                  │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─ knowledge (from past executions) ─────────────┐  │
│  │ [previous research] Thai restaurants in PDX...  │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
└──────────────────────────────────────────────────────┘
```

The agent is stateless. Every call, it sees the full picture — what the user said, what's come back, what's still pending, what's already known — and responds appropriately.

### Router Chaining

The router can chain tool calls without the agent knowing:

```
Agent: request("find Portland diners")
  │
  Router: search_web("Portland diners")
  │       → results arrive
  │       → Router notices user said "Thai" in conversation context
  │       → Router auto-fires: filter_results(cuisine="Thai")
  │       → Router auto-fires: get_reviews(filtered_results)
  │       → All three land in context store as one resolved block
  │
  Agent sees: "Here are 2 Thai diners with reviews"
              (never knew 3 tools ran)
```

### Router Responsibilities

```
1. Receive request from agent
2. Check knowledge base first — has this been answered before?
3. Pick tool(s) — maybe multiple in parallel
4. Generate passdown (mandatory for async)
5. Execute tools (all async, some just resolve fast)
6. As results arrive, push them to the context store
7. Rank results by relevance to current conversation
8. Expire old results that are no longer relevant
9. Decide: does this new result warrant notifying
   the user, or does it wait for their next message?
```

### Result Delivery

When an async result arrives, it lands in the context store with status `active`. It does NOT trigger an automatic LLM call. Instead, results are injected into the system prompt on the next LLM call:

```
System prompt (base):
  "You are a helpful assistant..."

System prompt (injected when pending results exist):
  "The following tool results have arrived since your last response:

   [request: search Portland diners]
   Result: Found 5 diners — Pok Pok (Thai), Screen Door (brunch)...

   Please acknowledge these results naturally in your next response."
```

No timing games, no idle detection, no race conditions. The result is just context for whenever the next call happens.

### Frontend Integration

The frontend uses the existing WS + HTTPS pattern:

```
useInteractiveChat(executionId)
  │
  ├── SSE stream tokens → dispatch(APPEND_TOKEN)     -- normal HTTPS streaming
  │
  └── WS event (tool result arrived) → dispatch(APPEND_MESSAGE)  -- async result
  │
  └── same messages[] state, same render cycle
```

React's setState batching is the concurrency gate. Both paths go through the same dispatch. No locks needed.

---

## Part 2: Agent-to-Agent Communication

### The Network

Each agent node runs in its own Docker container. The router isn't just picking tools — it's a **service mesh for agents**. It knows:

- **Node registry** — which agents are alive, who's busy
- **Capability index** — what each agent is good at (from persona/tools)
- **Context store** — what's already been figured out (shared knowledge)

### Routing to Agents

When the router receives a request, it can route to another agent instead of a tool:

```
Agent A (security auditor) is working
  │
  request("I need someone who understands database schemas")
  │
  Router checks:
  ├── Context store: has this been answered before? → use cached
  ├── Agent registry: who has DB expertise? → Agent B is idle
  ├── No specialist? → spawn a new agent with DB tools
  │
  Agent B responds → result lands in Agent A's context store
  │
  Agent A continues with the knowledge
```

Agent A never knew Agent B existed. The router matched intent to capability.

### Dynamic Swarms

The DAG currently defines fan-out statically. With the router, spawning becomes dynamic:

```
Agent A: request("analyze all 12 microservices for security issues")
  │
  Router: this is a fan-out job
  │
  ├── Spawn Agent B  → analyze auth-service
  ├── Spawn Agent C  → analyze payment-service
  ├── Spawn Agent D  → analyze user-service
  │   ...
  ├── All results land in context store
  │
  Agent A sees: "All 12 services analyzed, here's the summary"
```

The DAG didn't define this fan-out. The router decided it at runtime. The swarm is emergent, not preconfigured.

---

## Part 3: Context Nodes

### Living Indexes

Context nodes are dedicated LLMs that sit loaded with deep context on a specific topic. Other agents query them like a database through the router.

The agent doing security audit doesn't need to read every file. It asks the codebase context node: "what does the auth module do?" That node has already ingested the entire auth system — every file, every migration, every PR. It gives back a dense, relevant summary.

```
Working Agent (small context, focused on task)
  │
  request("what does the auth module do?")
  │
  ▼
Context Node: auth-system (huge context, all auth code loaded)
  │
  Returns: dense 500-word summary of exactly what's relevant
  │
  ▼
Working Agent sees one tool result, keeps working
```

The context node's response is just a tool result. It renders once in the conversation. The working agent stays lean — maximum signal, minimum tokens.

### Growth and Forking

Context nodes evolve. As agents ask questions and the node synthesizes answers, it gets better at summarizing that domain. You can:

- **Snapshot** a context node at a point in time
- **Fork** it — `auth-context-v3` knows everything `v2` knew plus the last sprint
- **Prune** by relevance — old context that's never queried expires
- **Merge** — combine two context nodes that have grown toward overlapping domains

### Agent Types in the Database

```
agents table:
  id: agent-009
  agent_type: 'worker' | 'context_node' | 'generator' | 'router'
  capabilities: ['auth-system', 'database-schema']
  status: 'idle' | 'loaded' | 'working'
  context_size: 85000  (tokens currently loaded)
  version: 3
  parent_id: agent-007  (forked from)
```

The frontend renders the whole network — which nodes are loaded, what they know, who's talking to who.

---

## Part 4: Generative Pipelines

### Agents That Produce Work

Agents can generate work that feeds other agents. Example — a file search pipeline:

```
Agent A needs codebase context
  │
  Router → search bot (generator type)
  │
  Search bot: runs file search, finds 40 relevant paths
  │
  Router: fan-out via for_each
  │
  ├── Context loader 1: reads files 1-10, summarizes
  ├── Context loader 2: reads files 11-20, summarizes
  ├── Context loader 3: reads files 21-30, summarizes
  ├── Context loader 4: reads files 31-40, summarizes
  │
  Results concatenated by relevance
  │
  Lands in context store → Agent A gets a dense summary
```

The agent asked one question. A whole pipeline ran.

### The Workflow Editor as Orchestrator UI

The existing schema supports this:

```
workflows → workflow_steps → workflow_step_edges
```

Steps can be tools, context nodes, generators, or agents — all nodes on a visual canvas. The frontend pulls from existing tables as building blocks:

- `tools` table → available tools (drag onto canvas)
- `agents` table → available agents and context nodes
- `output_schemas` table → the shape each step produces
- `prompt_templates` table → reusable prompts for steps

Creating a pipeline is assembling these parts visually, not writing code.

---

## Part 5: Collective Memory

### Execution History as Knowledge Base

Every `execution_messages` row is a question someone asked and an answer an LLM produced. Tool calls and their results. Reasoning chains. Summaries. Every agent that ever ran left a trail.

The router's decision tree:

```
1. Search knowledge base for existing answers
   Found good match? → return it (free, instant)

2. Search context store for loaded context
   Found? → return it (free, instant)

3. Neither? → route to agent/tool (costs tokens)
```

Every token spent teaching one agent becomes a free lookup for every agent after it. The system gets cheaper and faster the more it runs.

### V1: Tags and Filtered Queries (Build This First)

Don't start with embeddings. Start with the documents table you already have. Agents tag their research with explicit labels:

```sql
SELECT content FROM documents
WHERE tags @> ARRAY['NVDA', 'margins']
AND doc_type = 'research'
ORDER BY updated_at DESC
LIMIT 5
```

The "markdown module" — agents write structured research docs as they work. Other agents and humans can read them. This works today with what exists.

### V2: Embeddings and Semantic Search (Upgrade Path)

Add an embedding column to execution_messages:

```sql
ALTER TABLE execution_messages
ADD COLUMN embedding vector(1536);

CREATE INDEX idx_messages_embedding
ON execution_messages
USING ivfflat (embedding vector_cosine_ops);
```

Now any agent can search semantically. But embeddings alone are fuzzy. The metadata makes it reliable:

- **Freshness** — when was this produced?
- **Reuse count** — was this answer ever reused by another agent?
- **Provenance** — what task/workflow produced it?
- **Trust score** — was the output validated or corrected by a human?

The embedding finds candidates. The metadata ranks them.

### V3: Distillation

Raw execution messages get noisy. A distillation agent runs periodically and compresses raw history into durable knowledge:

```
Raw execution messages (millions, noisy, everything)
  │
  Distillation agent (runs periodically)
  │
  ├── What questions were asked most?
  ├── What answers were reused?
  ├── What became stale?
  │
  ▼
Knowledge base (thousands, dense, curated)
```

The markdown module is the output format. Living documents organized by topic that get better over time.

---

## Part 6: Stock Research (First Non-Code Use Case)

The data is public, the tools are well-defined, and results can be validated against reality.

### Example Workflow

```
Workflow: "Analyze NVDA for investment potential"

Step 1: Market data agent pulls financials, filings, price history
Step 2: News agent searches recent coverage, earnings calls
Step 3: Sector agent loads existing knowledge about semiconductors
        (from previous research — this is the memory paying off)
Step 4: Analysis agent gets dense context from steps 1-3
        writes findings to the markdown module
Step 5: Contrarian agent reads the analysis, pokes holes
Step 6: Final report agent synthesizes everything

Output: structured markdown doc in the "repo"
```

Next week: "Analyze AMD." Step 3 already has semiconductor context from the NVDA research. The system is faster and cheaper the second time. By the tenth company, the sector agent barely needs to do new work.

### What This Proves

The architecture is domain-agnostic. The same system that audits code can research stocks. The tools change, the workflows change, the knowledge base grows in a different direction — but the router, context store, agent mesh, and generative pipelines are identical infrastructure.

---

## Database Schema

### context_store

```sql
CREATE TABLE context_store (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id),
    source TEXT NOT NULL,          -- 'conversation' | 'tool_result' | 'pending' | 'system' | 'knowledge'
    priority REAL NOT NULL,        -- relevance score for ranking
    content TEXT NOT NULL,
    metadata JSONB,                -- tool name, request_id, agent_id, tags, etc.
    status TEXT NOT NULL DEFAULT 'active',  -- 'active' | 'expired' | 'pending'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ         -- optional TTL
);

CREATE INDEX idx_context_store_session ON context_store(session_id, status);
CREATE INDEX idx_context_store_priority ON context_store(session_id, priority DESC);
```

Before every LLM call:

```sql
SELECT * FROM context_store
WHERE session_id = ?
  AND status = 'active'
  AND (expires_at IS NULL OR expires_at > NOW())
ORDER BY priority DESC
LIMIT <token_budget>
```

That query IS the system prompt.

### router_requests

```sql
CREATE TABLE router_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id),
    agent_execution_id UUID REFERENCES agent_executions(id),
    intent TEXT NOT NULL,
    priority TEXT NOT NULL DEFAULT 'normal',
    callback_hint TEXT,
    routed_tool TEXT,              -- what the router chose (tool name or agent id)
    routed_args JSONB,
    is_async BOOLEAN NOT NULL DEFAULT FALSE,
    passdown TEXT,                 -- message for the user while waiting
    chain JSONB,                   -- follow-up calls if multi-step
    status TEXT NOT NULL DEFAULT 'pending',
    result TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
```

### Router Prompt

```
You are a tool routing agent. You receive natural-language requests
from a conversational AI and decide how to fulfill them.

## Your job

1. CHECK the knowledge base — has this been answered before?
2. CLASSIFY the request: trivial | simple | complex | multi-step
3. SELECT the best tool(s) or agent(s) from available resources
4. DECIDE execution mode:
   - sync:  Result is fast (<2s). Return result directly.
   - async: Result takes time. Return a passdown message.
5. COMPOSE the call with correct parameters
6. If multi-step, orchestrate a chain of calls yourself

## Response format

{
  "tool": "<tool_name or agent_id>",
  "tool_args": { ... },
  "async": true | false,
  "passdown": "<message for user>",   // REQUIRED if async: true
  "chain": [                           // optional follow-up calls
    { "tool": "...", "tool_args": { ... }, "condition": "on_success" }
  ]
}

## Available tools
<injected: all tool definitions from tools table>

## Available agents
<injected: agent registry with capabilities>

## Known knowledge
<injected: relevant cached results from knowledge base>
```

---

## Part 7: Dynamic Routers (Router-of-Routers)

### The Problem

Exposing multiple complex APIs to a single LLM is a context disaster. Each API has its own auth, pagination, query syntax, rate limits, and edge cases. Stuffing all of that into one system prompt produces confusion and hallucinated parameters.

### The Solution

Routers are tools. The main router doesn't need to know how Apollo's API works — it just knows "Agent B is the Apollo expert." Each API gets its own router with a focused system prompt containing only that API's documentation.

```
Agent: request("find customers in Seattle who work in warehousing")
  │
  Main Router: this is a CRM query → route to Apollo Router
  │
  ▼
Apollo Router (system prompt: full Apollo API docs, auth, search syntax)
  │
  Decides: search_people(location="Seattle", industry="Warehousing")
  │
  Executes → result lands in context store
  │
  Agent sees: "Found 47 contacts matching your criteria"
  (never knew two routers were involved)
```

The agent made one request. The main router identified the domain. The domain router knew the API. Clean separation at every layer.

### Why This Works

Each router is a **specialist with a small context**. The Apollo router's system prompt is 100% Apollo API docs — no noise from Salesforce, HubSpot, or internal tools. It makes better decisions because it's not confused by competing schemas.

The main router's job is simple classification: "what domain is this?" It doesn't need API details — just a capability index of which sub-routers handle what.

### Database Schema

```sql
CREATE TABLE tool_routers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,                    -- 'Apollo API Router'
    description TEXT,                      -- capability summary for the main router
    system_prompt TEXT NOT NULL,           -- full API docs, auth patterns, examples
    model_id TEXT NOT NULL,                -- which LLM runs this router
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tool_router_tools (
    router_id UUID NOT NULL REFERENCES tool_routers(id) ON DELETE CASCADE,
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (router_id, tool_id)
);
```

The main router sees sub-routers as capabilities in its registry. A sub-router sees only its own tools via the join table. The UI lets you drag tools onto routers the same way you drag tools onto agents.

### Chaining Depth

Nothing stops a sub-router from routing to another sub-router. Apollo Router could route to an enrichment router that cross-references LinkedIn. The depth is emergent, not hardcoded. In practice, 2-3 levels handles most real-world complexity.

```
Main Router
  ├── Code Router (file ops, git, test runners)
  ├── Research Router (web search, news, SEC filings)
  │     └── SEC Router (EDGAR API specialist)
  ├── CRM Router
  │     ├── Apollo Router (prospecting, contact search)
  │     └── Salesforce Router (deal pipeline, account data)
  └── Internal Router (company wiki, Slack search, Jira)
```

---

## Part 8: Chat as Pipeline

### The Product

The end user talks to an LLM. That LLM has the power of the entire system behind it — routers, sub-routers, agent swarms, context nodes, collective memory. The user doesn't know and doesn't care. They're just chatting.

### How It Works

A chat session is bound to a pipeline. The pipeline defines what the LLM can do — which routers, which tools, which knowledge domains. The LLM sees one tool: `request()`. Everything else is infrastructure.

```
┌─────────────────────────────────────────────────────────────┐
│                     CHAT SESSION                             │
│                                                             │
│  session.pipeline_id = pipe-042 (Stock Research Pipeline)   │
│                                                             │
│  User: "What's NVDA's margin trend over the last 4 quarters?" │
│                                                             │
│  LLM: request("NVDA quarterly margins, last 4 quarters")   │
│       passdown: "Pulling NVDA's financials now —            │
│                  anything specific beyond gross margins?"    │
│                                                             │
│  ┌─ VISIBLE TO USER ──────────────────────────────────────┐ │
│  │  💬 "Pulling NVDA's financials now..."                 │ │
│  │  ⏳ Fetching SEC filings...                            │ │
│  │  ⏳ Pulling market data...                             │ │
│  │  ✅ SEC filings loaded                                 │ │
│  │  ⏳ Analyzing margin trends...                         │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                             │
│  User: "Yeah include operating margins too"                 │
│  (user keeps talking while work runs)                       │
│                                                             │
│  LLM sees on next call:                                     │
│    - conversation history                                   │
│    - resolved results (SEC data, market data)               │
│    - in-flight (margin analysis still running)              │
│    - user's follow-up about operating margins               │
│                                                             │
│  LLM: "Here's what I've found so far..."                   │
│        + request("include operating margins in analysis")   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### The Session-Pipeline Binding

```sql
ALTER TABLE sessions ADD COLUMN pipeline_id UUID REFERENCES pipelines(id);
```

That's it. One column. A session with a `pipeline_id` is a powered chat. A session without one is a plain conversation. The frontend renders them differently:

- **Plain chat** — standard message thread
- **Powered chat** — message thread + live activity panel showing in-flight requests, resolved results, active agents

### What the User Sees

The chat interface has two zones:

```
┌──────────────────────────────┬─────────────────────────┐
│                              │                         │
│     CONVERSATION             │     ACTIVITY PANEL      │
│                              │                         │
│  You: What's NVDA's margin   │  ⏳ SEC filings         │
│       trend?                 │  ✅ Market data (2.1s)  │
│                              │  ⏳ Margin analysis     │
│  AI: Pulling financials...   │  ⏳ Peer comparison     │
│       any specifics?         │                         │
│                              │  ── completed ──        │
│  You: Include operating      │  ✅ News search (1.4s)  │
│       margins too            │                         │
│                              │  📊 Tokens: 4,200 in   │
│  AI: Here's what I found...  │            890 out     │
│                              │  💰 Cost: $0.02        │
│                              │                         │
│  [type a message...]         │                         │
└──────────────────────────────┴─────────────────────────┘
```

The conversation is fluid. The activity panel is informational. The user never has to wait in silence — passdowns keep the conversation moving, and the panel shows what's happening behind the scenes.

### Persistence and Growth

Every interaction writes to the same tables:

- `execution_messages` — the full conversation + tool calls
- `token_ledger` — cost per LLM call
- `documents` — research artifacts tagged by domain
- `context_store` — active context for the session

The next conversation about NVDA starts with knowledge. The router checks the knowledge base before spending tokens. "What were NVDA's margins?" — already answered, free lookup. "How do they compare to AMD?" — the semiconductor context node already has NVDA loaded, only AMD is new work.

The tenth stock research conversation costs a fraction of the first. The hundredth is almost free for questions that have been explored before. The system compounds.

### Creating Powered Chats

The UI for creating a new chat lets you pick a pipeline:

```
┌─ New Chat ──────────────────────────────────────┐
│                                                  │
│  Name: NVDA Deep Dive                           │
│                                                  │
│  Pipeline: [Stock Research Pipeline ▾]           │
│            ├── Code Analysis Pipeline            │
│            ├── Stock Research Pipeline            │
│            ├── Security Audit Pipeline            │
│            └── None (plain chat)                 │
│                                                  │
│  [Create]                                        │
└──────────────────────────────────────────────────┘
```

Picking a pipeline gives the chat access to that pipeline's full capability set — its routers, tools, agents, and knowledge domains. The LLM in the chat becomes the front-end to the entire system.

### Why This Is the Product

The pipeline is invisible. The router is invisible. The agent swarm is invisible. The sub-routers are invisible. The knowledge base is invisible. The user just talks to an AI that happens to be backed by a distributed system that gets better every time it runs.

That's the product: a chat interface where the thing you're talking to has real infrastructure behind it.

---

## Part 9 — Agent Rooms

A room is a shared conversation where multiple agents participate together, turn by turn, like a real meeting. The user talks to all of them at once. Each agent sees every prior message — including what the other agents said — and the conversation grows within each turn.

### The Concept

Today's interactive pipeline step is one agent, one conversation. Agent Rooms replace that with a group session. You put agents in a room, give each a system prompt that says "you are part of a greater conversation," and let them collaborate.

The key constraint: **sequential, not parallel.** Each agent speaks in order. Agent B sees Agent A's response before forming its own. This is what makes it collaboration instead of isolated parallel calls. The conversation context accumulates within a single user turn:

```
User: "Should we refactor the auth module?"

  → Security Agent sees: [user message]
  ← Security Agent: "The current implementation has three CVEs..."

  → Architecture Agent sees: [user message, security response]
  ← Architecture Agent: "Given those vulnerabilities, I'd restructure into..."

  → Code Agent sees: [user message, security response, architecture response]
  ← Code Agent: "Here's a migration plan based on both assessments..."
```

Each agent builds on what came before. The user gets a coordinated response from a team, not three isolated opinions.

### The Gatekeeper

Not every agent needs to speak on every turn. A lightweight gatekeeper agent (Haiku-tier) decides **who speaks and in what order** for each user message. It sees:

- The room's agent roster (names + roles)
- The conversation history
- The latest user message

It returns an ordered list of agents that should respond. Agents not on the list stay silent — no wasted tokens on irrelevant responses.

```
Gatekeeper prompt:
  "You manage a conversation room. Given the user's message and the
   agents available, return a JSON array of agent IDs in the order
   they should speak. Only include agents whose expertise is relevant.
   If only one agent is needed, return just that one."

Gatekeeper output:
  { "speakers": ["security-agent-id", "arch-agent-id"], "reason": "security review needed before architecture decisions" }
```

The alternative — having each agent self-filter with a "relevance score" — wastes tokens because every agent still gets called. The gatekeeper prevents that. One cheap call decides who speaks instead of N expensive calls that mostly say "nothing to add."

### Why Not Parallel

Parallel execution is faster but breaks collaboration. If three agents run simultaneously, none of them see each other's output. You get three independent responses that might contradict each other. Sequential execution is slower but produces coherent, building-on-each-other responses — like a real meeting where people speak one at a time.

The tradeoff is intentional. Rooms are for collaboration quality, not speed. Pipelines handle parallelism.

### How It Maps to Existing Infrastructure

Rooms don't need new tables — they're a pattern on top of what exists:

- **A room is a session** with multiple agents assigned (via a pipeline or direct config)
- **The gatekeeper is an agent** with a specific system prompt and Haiku model
- **Each agent turn is an agent_execution** linked to the session's stage_execution
- **Messages accumulate in execution_messages** — shared context is just the message history
- **The room roster** comes from the pipeline stage's agent assignments

The execution flow is:

1. User sends message to room session
2. Gatekeeper agent evaluates → returns speaker order
3. For each speaker in order:
   - Build prompt: agent's system prompt + full conversation history (including prior speakers this turn)
   - Call LLM
   - Append response to conversation history
   - Stream response to frontend via WS
4. All speakers done → turn complete, wait for next user message

### Room Configuration

A room is defined by:

- **Agents:** Which agents participate (from pipeline stage members or direct assignment)
- **Gatekeeper:** The agent that decides turn order (optional — without one, all agents speak every turn in fixed order)
- **Max speakers per turn:** Cap on how many agents respond to a single message (prevents runaway costs)
- **Turn strategy:** `gatekeeper` (LLM decides), `round-robin` (everyone every time), `explicit` (user tags who they want)

### The User Experience

The frontend renders a room as a group chat. Each agent's messages are visually distinct (name, avatar, color). The user sees the gatekeeper's routing as a subtle indicator — "Security Agent and Architecture Agent are responding..." — while agents stream their responses one after another.

The activity panel (from Chat as Pipeline, Part 8) shows the gatekeeper's decisions, token costs per agent, and which agents were skipped. Transparency into the collaboration process.

### Scaling Rooms

Rooms can be nested. A "lead" agent in one room might consult a sub-room of specialists before responding. The gatekeeper pattern applies recursively — each sub-room has its own gatekeeper deciding which specialists speak.

This connects to the delegation system (Part 2). An agent in a room delegates to a sub-room the same way it delegates to a sub-agent. The room is just a richer delegation target — instead of one agent processing a request, a coordinated group does.

---

## Summary

```
Layer 1 — Router:           Intent in, tool/agent selection out. Async with passdowns.
Layer 2 — Agent Mesh:       Agents route work to other agents. Dynamic swarms.
Layer 3 — Context Nodes:    Living indexes loaded with deep domain knowledge.
Layer 4 — Knowledge Base:   Execution history as searchable collective memory.
Layer 5 — Distillation:     Raw history compressed into curated, reusable knowledge.
Layer 6 — Dynamic Routers:  Routers route to routers. Each API gets a specialist.
Layer 7 — Chat as Pipeline: The user talks to an LLM backed by the full system.
Layer 8 — Agent Rooms:      Multiple agents collaborate turn-by-turn in shared context.

The workflow editor is the UI for assembling all of this visually.
The chat interface is the product — infrastructure is invisible.
Agent rooms turn single-agent chats into collaborative team sessions.
The system gets smarter and cheaper the more it runs.
The architecture is domain-agnostic — code, stocks, research, anything.
```
