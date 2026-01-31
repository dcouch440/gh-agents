# Orchestrator Context Flow

How the orchestrator processes a chat message through the tool-use loop with context management.

```
  User Message
       │
       ▼
  ┌─────────────────────────────────────────────┐
  │           ORCHESTRATOR (Sonnet)              │
  │                                              │
  │  ┌─────────────────────────────────────┐     │
  │  │  Context Budget Check               │     │
  │  │  estimated_chars > 480K? → STOP     │     │
  │  └──────────────┬──────────────────────┘     │
  │                 │                             │
  │                 ▼                             │
  │  ┌─────────────────────────────────────┐     │
  │  │  LLM Call (Sonnet)                  │     │
  │  │  ├─ RetryingProvider wraps call     │     │
  │  │  ├─ 429? → backoff + retry (5x)    │     │
  │  │  └─ retry-after header respected    │     │
  │  └──────────────┬──────────────────────┘     │
  │                 │                             │
  │          ┌──────┴──────┐                      │
  │          │             │                      │
  │       EndTurn      ToolUse                    │
  │          │             │                      │
  │          ▼             ▼                      │
  │        Done    ┌──────────────┐               │
  │                │ Execute Tool │               │
  │                └──────┬───────┘               │
  │                       │                       │
  │         ┌─────────────┼─────────────┐         │
  │         ▼             ▼             ▼         │
  │   ┌──────────┐ ┌───────────┐ ┌──────────┐    │
  │   │read_file │ │search_files│ │ others   │    │
  │   └────┬─────┘ └─────┬─────┘ └────┬─────┘    │
  │        │              │            │          │
  │        ▼              │            │          │
  │  ┌───────────┐        │            │          │
  │  │ >2K chars?│        │            │          │
  │  │           │        │            │          │
  │  │ YES → ┌───────┐   │            │          │
  │  │       │ Haiku │   │            │          │
  │  │       │ reads │   │            │          │
  │  │       │  and  │   │            │          │
  │  │       │summar-│   │            │          │
  │  │       │ izes  │   │            │          │
  │  │       └───┬───┘   │            │          │
  │  │           │        │            │          │
  │  │ NO → raw  │        │            │          │
  │  │ content   │        │            │          │
  │  └───────────┘        │            │          │
  │        │              │            │          │
  │        ▼              ▼            ▼          │
  │  ┌─────────────────────────────────────┐     │
  │  │  Result Processing                  │     │
  │  │  ├─ Compact JSON (no pretty-print)  │     │
  │  │  └─ Truncate at 10K chars           │     │
  │  └──────────────┬──────────────────────┘     │
  │                 │                             │
  │          200ms pause                          │
  │                 │                             │
  │                 ▼                             │
  │          Loop back to Context Budget Check    │
  │          (max 10 rounds)                      │
  │                                              │
  └─────────────────────────────────────────────┘
       │
       ▼
  Stream Response to Client (SSE)
```

## Guardrails

| Layer | Protection | Location |
|-------|-----------|----------|
| Context budget | Breaks loop if estimated context > 120K tokens (~480K chars) | `orchestrator.rs` top of loop |
| RetryingProvider | Retries 429s with exponential backoff (5 attempts), respects retry-after header | `llm/retry.rs` wrapping provider |
| Inter-round delay | 200ms pause between tool rounds to avoid burst API calls | `orchestrator.rs` after tool results |
| Haiku file reads | Files > 2K chars summarized by Haiku before entering Sonnet context | `tools.rs` read_file |
| Result truncation | Individual tool results capped at 10K chars | `orchestrator.rs` result processing |
| Compact JSON | `to_string` instead of `to_string_pretty` saves whitespace tokens | `orchestrator.rs` result processing |
| search_files | Grep-based search returns line matches instead of requiring full file reads | `tools.rs` new tool |

## Key files

- `src/server/orchestrator.rs` — Tool-use loop, context budget, result processing
- `src/server/tools.rs` — Tool definitions and handlers (read_file, search_files, haiku_read_file)
- `src/llm/retry.rs` — RetryingProvider with exponential backoff
- `src/llm/anthropic.rs` — retry-after header parsing
- `src/llm/types.rs` — Message.estimated_chars() for budget tracking
