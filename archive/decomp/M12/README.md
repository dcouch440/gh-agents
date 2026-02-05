# Milestone 12: Terminal CLI (Ink)

> Claude Code-style terminal interface for nexor, built with TypeScript + Ink.

## Goal

Interactive terminal CLI that connects to the Rust backend. Chat with the orchestrator directly in your terminal — streaming responses, markdown rendering, same UX as Claude Code.

**Checkpoint**: Can run `nexor` in terminal, authenticate, send messages, see streaming responses with markdown formatting.

---

## Context

The original M12 was React browser features. This replaces it with a terminal-native CLI using Ink (React for CLIs). The React web UI remains in `ui/` as an alternative.

**Architecture**:
```
┌─────────────────────────────────────┐
│          Terminal (Ink)              │
│  ┌───────────┐  ┌────────────────┐  │
│  │  Input    │  │  MessageList   │  │
│  └───────────┘  └────────────────┘  │
├─────────────────────────────────────┤
│          API Client (HTTP + SSE)    │
├─────────────────────────────────────┤
│          Rust Backend (M10)         │
└─────────────────────────────────────┘
```

**Key constraint**: Backend already exists. CLI is a pure client — no backend changes needed.

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 12.1 | CLI Scaffolding | 4 | M10 (server) |
| 12.2 | API Client | 4 | 12.1 |
| 12.3 | Auth Flow | 3 | 12.2 |
| 12.4 | Chat UI Components | 5 | 12.3 |
| 12.5 | Streaming & SSE | 3 | 12.4 |
| 12.6 | Polish & Integration | 3 | 12.5 |

---

## Design Notes

Match Claude Code's terminal UX:
- Flat turn-based conversation (no bubbles, no avatars)
- Role labels ("You" / "nexor") with timestamps
- Streaming text token-by-token
- Markdown rendered in terminal (headers, code blocks, lists)
- Minimal chrome — content first
- Input at the bottom, conversation scrolls above

---

## Completion Criteria

- [ ] `npx nexor` launches the CLI
- [ ] Can authenticate with the backend
- [ ] Can send messages and see streaming responses
- [ ] Markdown renders correctly in terminal
- [ ] Code blocks display with syntax info
- [ ] Makefile targets for CLI dev/build
