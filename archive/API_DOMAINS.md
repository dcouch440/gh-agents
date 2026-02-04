# API Domains Reference

This document provides an overview of every API domain in the nexor application, what it handles, and how it's used.

## Core Infrastructure

### `health`
**Purpose:** System health monitoring
**What it does:** Provides health check endpoint that returns server status, version, and database connectivity
**Used for:** Monitoring uptime, debugging connection issues, service discovery
**Key endpoints:** `GET /api/health`

### `config`
**Purpose:** Application configuration management
**What it does:** Manages server settings including verbosity level, agent pool configuration, autonomy level, git strategy, and sandbox mode
**Used for:** Adjusting runtime behavior, configuring agent pools, setting operational modes
**Key endpoints:** `GET /api/config`, `PATCH /api/config`

### `auth`
**Purpose:** Authentication and authorization
**What it does:** Handles user registration, login, password setup, and token-based authentication
**Used for:** Securing API endpoints, managing user sessions, first-time setup
**Key endpoints:** `POST /api/auth/setup`, `POST /api/auth/login`, `POST /api/auth/register`, `GET /api/auth/me`

## Agent Management

### `agents`
**Purpose:** Agent lifecycle management
**What it does:** CRUD operations for AI agents including creation, configuration, status tracking, and deletion
**Used for:** Creating worker/orchestrator/utility agents, configuring their models and personas, monitoring agent pool statistics
**Key endpoints:** `GET /api/agents`, `POST /api/agents`, `GET /api/agents/:id`, `PATCH /api/agents/:id`, `DELETE /api/agents/:id`

### `agent_context`
**Purpose:** Agent knowledge base linkage
**What it does:** Associates documents with agents to provide contextual knowledge during execution
**Used for:** Giving agents access to specific documentation, code examples, or reference materials
**Key endpoints:** `GET /api/agents/:id/context`, `PUT /api/agents/:id/context`

### `agent_executions`
**Purpose:** Agent execution tracking and interaction
**What it does:** Manages individual agent execution instances, tracks status, handles interactive chat with running agents, and approval flows
**Used for:** Monitoring agent work, debugging execution issues, interactive agent collaboration, approving agent actions
**Key endpoints:** `GET /api/agent-executions/:id`, `POST /api/agent-executions/:id/messages`, `GET /api/agent-executions/:id/messages`, `POST /api/agent-executions/:id/approve`

### `cancellation`
**Purpose:** Agent execution control
**What it does:** Provides ability to cancel running agent executions
**Used for:** Stopping runaway agents, terminating stuck executions, resource cleanup
**Key endpoints:** `POST /api/agent-executions/:id/cancel`

## Task & Work Management

### `tasks`
**Purpose:** Task lifecycle management
**What it does:** CRUD operations for tasks including creation, listing with filters, priority management, and status tracking
**Used for:** Breaking down work into units, assigning tasks to agents, tracking completion, filtering by status/priority
**Key endpoints:** `GET /api/tasks`, `POST /api/tasks`, `GET /api/tasks/:id`

### `workflows`
**Purpose:** Multi-step workflow orchestration
**What it does:** Manages workflows composed of sequential/parallel steps, each step assigned to an agent with specific prompts and output schemas
**Used for:** Defining complex multi-agent processes, workflow automation, structured data processing
**Note:** Workflows have replaced the deprecated pipeline system (removed Feb 3, 2026). Use Workflow Collections for equivalent functionality with improved architecture.
**Key endpoints:**
- Workflows: `GET /api/workflows`, `POST /api/workflows`, `GET /api/workflows/:id`, `DELETE /api/workflows/:id`
- Steps: `POST /api/workflows/:id/steps`, `GET /api/workflows/:id/steps`, `PATCH /api/workflows/:wid/steps/:sid`
- Edges: `POST /api/workflows/:id/edges`, `GET /api/workflows/:id/edges`, `DELETE /api/workflows/:id/edges`
- Documents: `POST /api/workflows/:wid/steps/:sid/documents`

## Communication & Interaction

### `chat`
**Purpose:** Real-time chat interface with orchestrator
**What it does:** Handles chat message sending, streaming responses via SSE, message history retrieval with pagination
**Used for:** User interaction with the AI system, debugging conversations, chat history review
**Key endpoints:** `POST /api/chat`, `GET /api/chat/stream`, `GET /api/chat/history`, `DELETE /api/chat/history`

### `sessions`
**Purpose:** Conversation session management
**What it does:** Manages isolated chat sessions with specific agents/modes, tracks session history, supports multiple concurrent conversations
**Used for:** Context-isolated conversations, testing different agent configurations, managing multiple workstreams
**Key endpoints:** `GET /api/sessions`, `POST /api/sessions`, `GET /api/sessions/:id`, `POST /api/sessions/:id/chat`, `GET /api/sessions/:id/history`

### `session_context`
**Purpose:** Session-level context storage
**What it does:** Stores and retrieves context entries and router requests for specific sessions
**Used for:** Tracking what information is available in a session, debugging tool routing decisions
**Key endpoints:** `GET /api/sessions/:id/context`, `GET /api/sessions/:id/requests`

### `rooms`
**Purpose:** Multi-agent collaboration spaces
**What it does:** Creates virtual rooms where multiple agents can interact, with gatekeeper control, turn management, and role assignments
**Used for:** Agent-to-agent collaboration, round-robin discussions, panel-style problem solving
**Key endpoints:** `POST /api/rooms`, `GET /api/rooms/:id`, `POST /api/rooms/:id/members`, `POST /api/rooms/:id/sessions/:sid/messages`

## Tools & Capabilities

### `tools`
**Purpose:** Tool definition and management
**What it does:** CRUD operations for tools (functions agents can call), manages tool schemas and parameters
**Used for:** Defining available capabilities, managing tool library, versioning tool definitions
**Key endpoints:** `GET /api/tools`, `POST /api/tools`, `GET /api/tools/:id`, `PATCH /api/tools/:id`, `DELETE /api/tools/:id`

### `tool_routers`
**Purpose:** Intelligent tool routing
**What it does:** Manages tool routers that use LLMs to intelligently select and route to appropriate tools based on user requests
**Used for:** Dynamic tool selection, intent classification, tool orchestration
**Key endpoints:** `GET /api/tool-routers`, `POST /api/tool-routers`, `GET /api/tool-routers/:id`, `PUT /api/tool-routers/:id/tools`

## Knowledge & Templates

### `documents`
**Purpose:** Document storage and management
**What it does:** Manages documents with content, tags, summaries, and search capabilities
**Used for:** Storing reference materials, code snippets, design docs, search across knowledge base
**Key endpoints:** `GET /api/documents`, `POST /api/documents`, `GET /api/documents/:id`, `GET /api/documents/search`

### `prompt_templates`
**Purpose:** Reusable prompt management
**What it does:** CRUD operations for prompt templates used in workflow steps
**Used for:** Standardizing agent instructions, version control for prompts, template reuse across workflows
**Key endpoints:** `GET /api/prompt-templates`, `POST /api/prompt-templates`, `GET /api/prompt-templates/:id`

### `output_schemas`
**Purpose:** Structured output definition
**What it does:** Manages JSON schemas that define expected output formats for agent executions
**Used for:** Enforcing structured responses, data validation, workflow step contracts
**Key endpoints:** `GET /api/output-schemas`, `POST /api/output-schemas`, `GET /api/output-schemas/:id`

## Data & Analytics

### `results`
**Purpose:** Structured output storage
**What it does:** Stores and retrieves structured data produced by agent executions, linked to output schemas
**Used for:** Collecting workflow outputs, querying execution results, data aggregation
**Key endpoints:** `GET /api/results`, `GET /api/results/:id`, `DELETE /api/results/:id`

### `costs`
**Purpose:** Cost tracking and reporting
**What it does:** Tracks API costs by model, provides spend breakdowns, supports time-range filtering
**Used for:** Budget monitoring, cost attribution, usage analytics
**Key endpoints:** `GET /api/costs`

## Domain Relationships

```
User Authentication (auth)
    └─> Creates/manages Agents (agents)
        ├─> Configured with Tools (tools) via tool assignment
        ├─> Enriched with Documents (documents) via agent_context
        ├─> Executed in Workflows (workflows)
        │   ├─> Uses Prompt Templates (prompt_templates)
        │   ├─> Produces Results (results) matching Output Schemas (output_schemas)
        │   └─> Tracked via Agent Executions (agent_executions)
        ├─> Participates in Rooms (rooms) for multi-agent collaboration
        └─> Interacts via Chat (chat) or Sessions (sessions)
            └─> Context tracked in Session Context (session_context)

Tasks (tasks) can be worked on by any execution flow above.

Configuration (config) affects global behavior across all domains.

Tool Routers (tool_routers) provide intelligent tool selection.

Costs (costs) tracks spending across all LLM operations.

Cancellation (cancellation) can halt any running execution.

Health (health) monitors entire system.
```

## Common Patterns

### CRUD Resources
Most domains follow standard REST patterns:
- `GET /api/{domain}` - List all
- `POST /api/{domain}` - Create new
- `GET /api/{domain}/:id` - Get single
- `PATCH /api/{domain}/:id` - Update
- `DELETE /api/{domain}/:id` - Delete

### Authentication
All endpoints (except `/api/health` and `/api/auth/setup`) require Bearer token authentication obtained via login.

### Filtering & Pagination
List endpoints typically support query parameters:
- `limit` - Maximum results to return
- `offset` - Skip first N results
- `status` - Filter by status field
- `since` - Filter by timestamp

### Versioning
Many entities track version numbers for optimistic concurrency control.

## Use Case Examples

### Setting up a simple workflow
1. Create agents via `agents` domain
2. Define tools via `tools` domain
3. Assign tools to agents
4. Create prompt templates via `prompt_templates`
5. Create output schemas via `output_schemas`
6. Build workflow via `workflows` domain
7. Execute and monitor via `agent_executions`
8. Retrieve results via `results`

### Interactive agent chat
1. Authenticate via `auth`
2. Send message via `chat`
3. Stream response via `chat/stream`
4. Review history via `chat/history`

### Multi-agent collaboration
1. Create room via `rooms`
2. Add agent members with roles
3. Create room session
4. Send messages and monitor transcript
5. Agents interact based on turn management

### Cost monitoring
1. Run various agent executions
2. Query costs via `costs` domain
3. Filter by date range
4. Review per-model breakdown
