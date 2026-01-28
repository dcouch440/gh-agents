# Milestone 2: LLM Layer

> Can send prompts to Anthropic and get streaming responses.

## Goal

Establish the LLM integration layer for nexor. After this milestone, a developer can:
- Send prompts to Claude via the Anthropic API
- Receive streaming token responses in real-time
- Track token usage and calculate costs per API call
- Handle rate limits and transient failures gracefully

**Checkpoint**: Can chat with Claude via CLI, see tokens stream in, see cost tracked.

---

## Tickets

| Ticket | Title | Slices | Dependencies | Est. Complexity |
|--------|-------|--------|--------------|-----------------|
| 2.1 | Provider Abstraction | 3 | M1 types | Low |
| 2.2 | Anthropic Client | 4 | 2.1 | High |
| 2.3 | Cost Tracking | 3 | 2.2 | Medium |
| 2.4 | Retry Logic | 3 | 2.2 | Medium |

**Total Slices**: 13

---

## Dependency Graph

```
[M1: Foundation] ──► [2.1 Provider Abstraction]
                            │
                            ▼
                     [2.2 Anthropic Client]
                            │
               ┌────────────┼────────────┐
               │            │            │
               ▼            ▼            │
        [2.3 Cost      [2.4 Retry        │
         Tracking]      Logic]           │
               │            │            │
               └────────────┴────────────┘
                            │
                            ▼
                   [Milestone Complete]
```

**Simplified view:**

```
M1 ──► 2.1 ──► 2.2 ──┬──► 2.3
                     │
                     └──► 2.4
```

---

## Parallelization

**Can run in parallel:**
- 2.1 must complete first (defines traits/types)
- 2.2 must complete second (implements the provider)
- Then: 2.3 and 2.4 can run simultaneously (both depend on 2.2)

**Optimal execution order:**
1. Start with 2.1 (provider abstraction)
2. After 2.1: Start 2.2 (Anthropic client)
3. After 2.2: Start 2.3 and 2.4 in parallel

**Agent tier recommendations:**
| Ticket | Recommended Tier | Reason |
|--------|------------------|--------|
| 2.1 | Worker | Trait design needs care |
| 2.2 | Worker | HTTP/SSE parsing is complex |
| 2.3 | Worker | Database integration |
| 2.4 | Worker | Error handling patterns |

---

## File Changes Summary

### New Files Created

```
nexor/
├── src/
│   └── llm/
│       ├── mod.rs                      ← 2.1.1 (expand from placeholder)
│       ├── provider.rs                 ← 2.1.1, 2.1.2, 2.1.3
│       ├── types.rs                    ← 2.1.2, 2.1.3
│       ├── anthropic.rs                ← 2.2.1, 2.2.2, 2.2.3, 2.2.4
│       ├── cost.rs                     ← 2.3.1, 2.3.2, 2.3.3
│       └── retry.rs                    ← 2.4.1, 2.4.2, 2.4.3
└── tests/
    └── llm_integration.rs              ← 2.2.2 (integration tests)
```

### Dependencies to Add

```toml
# Cargo.toml additions for M2
reqwest = { version = "0.12", features = ["json", "stream"] }
futures = "0.3"
async-stream = "0.3"
tokio-stream = "0.1"
```

---

## Verification Checklist

After all tickets complete, verify:

- [ ] `cargo check` passes with no errors
- [ ] `cargo test` passes for all llm modules
- [ ] Can make authenticated request to Anthropic API
- [ ] Non-streaming `send_message()` returns complete response
- [ ] Streaming responses parse SSE events correctly
- [ ] Token counts extracted from responses
- [ ] Cost records written to database
- [ ] `get_summary()` aggregates costs correctly
- [ ] Retry logic respects 429/5xx status codes
- [ ] Exponential backoff increases correctly
- [ ] Max retries configuration is respected

---

## Environment Requirements

Before testing M2, ensure:

```bash
# API key must be set for integration tests
export ANTHROPIC_API_KEY="sk-ant-..."
```

---

## Notes

- This milestone establishes the LLM communication layer
- Anthropic is the only provider for v1.0 (trait allows future expansion)
- Streaming is critical for TUI responsiveness (tokens appear as received)
- Cost tracking enables budget awareness and per-task cost attribution
- Retry logic is essential for production reliability

---

## Next Milestone

After M2, proceed to:
- **M3: Agent Runtime** - Uses LLM layer for agent task execution
- **M4: Prompt Engineering** - Designs prompts that will use this layer
