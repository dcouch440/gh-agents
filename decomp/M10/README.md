# Milestone 10: Server Layer

> Axum HTTP server exposing REST API and WebSocket for React frontend

## Goal

A working HTTP server that exposes the existing orchestration core via REST API and WebSocket.

**Checkpoint**: Can curl `/api/health`, send a chat message via API, receive streaming updates via WebSocket.

---

## Context

This milestone bridges the Rust backend (M1-M9) with the new React frontend (M11-M13). The orchestration core is complete - we're adding a web layer on top.

**Architecture**:
```
React Frontend (M11-M13)
         │
         ▼ HTTP + WebSocket
┌─────────────────────────────────────────────┐
│            Axum Server (M10)                │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐      │
│  │REST API │ │WebSocket│ │  Auth   │      │
│  └─────────┘ └─────────┘ └─────────┘      │
└─────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────┐
│     Existing Orchestration Core (M1-M9)     │
└─────────────────────────────────────────────┘
```

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 10.1 | Axum Server Setup | 4 | None |
| 10.2 | REST API - Core Endpoints | 5 | 10.1 |
| 10.3 | REST API - Chat Endpoint | 4 | 10.1, 10.2 |
| 10.4 | WebSocket Gateway | 5 | 10.1 |
| 10.5 | Authentication | 5 | 10.1, 10.2 |
| 10.6 | Static File Serving | 3 | 10.1 |

---

## New Dependencies

Add to `Cargo.toml`:

```toml
# HTTP Server
axum = "0.7"
tower-http = { version = "0.5", features = ["cors", "fs", "trace"] }

# WebSocket
tokio-tungstenite = "0.21"
axum-extra = { version = "0.9", features = ["typed-header"] }

# Auth
argon2 = "0.5"
jsonwebtoken = "9"
```

---

## New File Structure

```
src/server/
├── mod.rs           # Server entry point, router assembly
├── api.rs           # REST endpoint handlers
├── ws.rs            # WebSocket handler
├── auth.rs          # Authentication middleware and handlers
├── state.rs         # Shared application state (AppState)
└── extractors.rs    # Custom Axum extractors
```

---

## Code to Remove

Before starting this milestone, delete the TUI code:

```bash
rm -rf src/tui/
```

Update `src/lib.rs` to remove `pub mod tui;`

Update `Cargo.toml` to remove:
- `ratatui`
- `crossterm`
- `syntect`

---

## Completion Criteria

- [ ] Server starts on configurable port
- [ ] CORS configured for local development
- [ ] All core REST endpoints working
- [ ] WebSocket connects and broadcasts
- [ ] Local auth with password hash
- [ ] JWT tokens issued and validated
- [ ] Static files served in production mode
- [ ] Graceful shutdown on SIGTERM
