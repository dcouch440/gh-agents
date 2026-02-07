# Nexor Backend Architecture Overview

Quick reference for navigating the system documentation.

---

## Document Index

| # | Document | Covers |
|---|----------|--------|
| 01 | [API Reference](./01-api-reference.md) | All REST endpoints, routes, request/response types, status codes |
| 02 | [Hub, Engine & Strategies](./02-hub-engine-strategies.md) | Execution engine, strategy pattern, 7 filters, 6 strategies, DAG executor, mode resolver |
| 03 | [State, WebSocket, Auth & Tools](./03-state-websocket-auth-tools.md) | AppState, EventBus, real-time WS events, JWT auth, tool system |
| 04 | [Database Schema](./04-database-schema.md) | All tables, columns, relationships, enums, execution envelopes |
| 05 | [Executors & LLM Providers](./05-executors-llm-providers.md) | Chat/Collection/Room executors, Anthropic/Ollama clients, retry/rate-limit |

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         React Frontend                              │
│                     (Vite, TypeScript, MUI)                          │
└───────────────┬───────────────────────────────┬─────────────────────┘
                │ REST API                       │ WebSocket
                v                                v
┌───────────────────────────────┐  ┌──────────────────────────────────┐
│       Axum HTTP Server        │  │       WebSocket Handler          │
│  (JWT Auth, CORS, Routes)     │  │  (Topic subscriptions, events)   │
└───────────────┬───────────────┘  └──────────────┬───────────────────┘
                │                                  │
                v                                  │
┌───────────────────────────────┐                  │
│      API Handlers (25+)       │                  │
│  agents, workflows, rooms,    │                  │
│  chat, executions, etc.       │                  │
└───────────────┬───────────────┘                  │
                │                                  │
                v                                  │
┌───────────────────────────────────────────────────────────────────┐
│                        AppState (Arc)                              │
│  repos(14), events, config, providers, mode_resolver, streams     │
└───────┬──────────┬────────────┬───────────────┬──────────────────┘
        │          │            │               │
        v          v            v               v
┌──────────┐ ┌──────────┐ ┌──────────┐  ┌──────────────────┐
│ Chat     │ │Collection│ │  Room    │  │    EventBus      │
│ Executor │ │DAG Exec  │ │ Executor │  │ (broadcast 256)  │
└────┬─────┘ └────┬─────┘ └────┬─────┘  └───────┬──────────┘
     │            │             │                 │
     v            v             v                 v
┌───────────────────────────────────────┐  ┌──────────────┐
│           Hub (Unified Engine)        │  │  WebSocket   │
│                                       │  │  Clients     │
│  ┌─────────────────────────────────┐  │  └──────────────┘
│  │  ExecutionEngine                │  │
│  │  (LLM loop, tools, streaming)  │  │
│  └──────────────┬──────────────────┘  │
│                 │                      │
│  ┌──────────────v──────────────────┐  │
│  │  Strategies                     │  │
│  │  Chat | DagStep | Router |     │  │
│  │  Interactive | Room | Cavernous│  │
│  └──────────────┬──────────────────┘  │
│                 │                      │
│  ┌──────────────v──────────────────┐  │
│  │  Filters (7)                    │  │
│  │  SchemaValidation | FewShot |  │  │
│  │  Reasoning | Recovery |        │  │
│  │  Enhancement | Guidance |      │  │
│  │  DebateVerification            │  │
│  └─────────────────────────────────┘  │
└──────────────────┬────────────────────┘
                   │
                   v
┌──────────────────────────────────────────┐
│         LLM Provider Stack               │
│                                          │
│  RateLimitedProvider (10 concurrent)     │
│    -> RetryingProvider (5 retries)       │
│      -> AnthropicClient / OllamaClient  │
└──────────────────┬───────────────────────┘
                   │
                   v
┌──────────────────────────────────────────┐
│            PostgreSQL                     │
│  60+ tables, 14 repository traits        │
└──────────────────────────────────────────┘
```

---

## Key Concepts

### Execution Tiers (Workflow Steps)
| Tier | Mode | Description |
|------|------|-------------|
| 1 | `single` | One agent executes once |
| 2 | `for_each` | Iterate array items, optional label-based routing to specialist agents |
| 3 | `cavernous` | 2-phase document-based dynamic routing |
| 4 | `room` | Multi-agent room discussion with gatekeeper |

### Data Flow Through DAG
```
Step A outputs StepExecutionEnvelope
  -> Stored in step_outputs map
  -> Downstream Step B declares input ports
  -> Port resolution extracts via json_path from Step A's envelope.data
  -> Injected into Step B's prompt template as {port_name}
```

### Streaming Architecture
- **Chat**: SSE via buffered response streams (late-join replay)
- **Rooms**: WebSocket events (SpeakerToken, SpeakerEnd, TurnComplete)
- **Workflows**: WebSocket events (StepStarted, StepCompleted, ForEachProgress)
- **DAG steps**: Non-streaming (batch execution, full response recorded)

### Authentication
- JWT tokens (HS256, 24-hour expiry)
- Bearer header or `?token=` query param (for SSE/WS)
- User ownership enforced on all resource operations

### Cost Tracking
Every LLM call recorded in `token_ledger`:
- Opus: $15/$75 per 1M tokens (in/out)
- Sonnet: $3/$15
- Haiku: $0.25/$1.25
- Local (Ollama): $0.00

---

## Quick Command Reference

```bash
# Backend
~/.cargo/bin/cargo check          # Type check
~/.cargo/bin/cargo test           # All tests
~/.cargo/bin/cargo run            # Run server

# Frontend (from frontend/)
npx tsc --noEmit                  # Type check
npx eslint .                      # Lint
npx vite build                    # Build

# Database
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "SELECT 1;"
```
