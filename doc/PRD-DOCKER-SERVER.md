# nexor Server Mode PRD

> Docker-Based Local Server Architecture for Distribution & Accounts

**Epic**: Server Mode & Distribution
**Status**: Draft
**Author**: AI Assistant
**Date**: 2026-01-27

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Problem Statement](#problem-statement)
3. [Goals & Non-Goals](#goals--non-goals)
4. [Architecture Overview](#architecture-overview)
5. [Deployment Models](#deployment-models)
6. [Authentication & Accounts](#authentication--accounts)
7. [Data Architecture](#data-architecture)
8. [API Design](#api-design)
9. [Web UI](#web-ui)
10. [Docker Configuration](#docker-configuration)
11. [Distribution Strategy](#distribution-strategy)
12. [Pricing & Licensing](#pricing--licensing)
13. [Security Considerations](#security-considerations)
14. [Migration Path](#migration-path)
15. [Implementation Phases](#implementation-phases)
16. [Open Questions](#open-questions)

---

## Executive Summary

Transform nexor from a local TUI application into a Docker-based server that can be:

1. **Self-hosted** - Users run `docker run nexor` on their machine
2. **Distributed** - Downloaded as a single command, auto-updates
3. **Account-enabled** - Optional accounts for licensing, sync, and premium features

The server exposes an HTTP API and WebSocket interface, enabling:
- Web UI access from any browser
- Original TUI as a thin client
- Multi-device access on local network
- Future path to cloud-hosted offering

---

## Problem Statement

### Current Limitations

| Issue | Impact |
|-------|--------|
| Binary distribution | Complex cross-platform builds, manual updates |
| Local-only state | No sync, no backup, device-locked |
| No monetization path | Can't sustain development |
| TUI-only interface | Limits audience to terminal users |
| Single-device | Can't access from phone/tablet |

### User Needs

1. **Developers** want easy installation (`docker run` vs download + chmod + PATH)
2. **Teams** want shared state and collaborative workflows
3. **Enterprise** wants audit trails, SSO, and support contracts
4. **Mobile users** want to monitor agents from phone

---

## Goals & Non-Goals

### Goals

- [ ] One-command installation via Docker
- [ ] Automatic updates with version pinning option
- [ ] Web UI for browser-based access
- [ ] Optional account system for premium features
- [ ] Local-first: works fully offline
- [ ] Data persists across container restarts
- [ ] Secure by default (localhost-only, auth required)

### Non-Goals (v1)

- Cloud-hosted multi-tenant SaaS (future)
- Mobile native apps (web responsive is sufficient)
- Real-time collaboration (single user per instance)
- Kubernetes/orchestration support (future)

---

## Architecture Overview

### High-Level Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        User's Machine                                │
│                                                                      │
│  ┌──────────────┐    ┌──────────────────────────────────────────┐   │
│  │   Browser    │───▶│           Docker Container                │   │
│  │   (Web UI)   │    │  ┌────────────────────────────────────┐  │   │
│  └──────────────┘    │  │         nexor-server               │  │   │
│                      │  │  ┌──────────┐  ┌──────────────┐    │  │   │
│  ┌──────────────┐    │  │  │ HTTP API │  │  WebSocket   │    │  │   │
│  │   TUI Client │───▶│  │  │  :3000   │  │  (real-time) │    │  │   │
│  │   (optional) │    │  │  └──────────┘  └──────────────┘    │  │   │
│  └──────────────┘    │  │         │              │           │  │   │
│                      │  │  ┌──────┴──────────────┴───────┐   │  │   │
│                      │  │  │      Orchestration Core     │   │  │   │
│                      │  │  │  (agents, tasks, LLM calls) │   │  │   │
│                      │  │  └─────────────┬───────────────┘   │  │   │
│                      │  │                │                   │  │   │
│                      │  │  ┌─────────────▼───────────────┐   │  │   │
│                      │  │  │         SQLite              │   │  │   │
│                      │  │  │    /data/nexor.db           │   │  │   │
│                      │  │  └─────────────────────────────┘   │  │   │
│                      │  └────────────────────────────────────┘  │   │
│                      │                   │                       │   │
│                      │  Volumes:         │                       │   │
│                      │  ├─ /data ────────┘ (SQLite, config)     │   │
│                      │  └─ /workspace ───── (mounted codebase)  │   │
│                      └──────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    ~/projects/my-app                          │   │
│  │                    (your codebase)                            │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼ (optional, for accounts)
                    ┌───────────────────────┐
                    │   nexor.dev API       │
                    │   (license, sync)     │
                    └───────────────────────┘
```

### Component Breakdown

| Component | Technology | Purpose |
|-----------|------------|---------|
| **HTTP Server** | Axum (Rust) | REST API, static file serving |
| **WebSocket** | tokio-tungstenite | Real-time updates, streaming |
| **Web UI** | Leptos or static SPA | Browser interface |
| **Database** | SQLite | State, history, analytics |
| **Auth** | JWT + optional OAuth | Local auth or account linking |
| **Container** | Docker | Packaging, isolation |

---

## Deployment Models

### Model 1: Self-Hosted (Primary)

User runs nexor entirely on their machine.

```bash
# Quick start
docker run -d \
  --name nexor \
  -p 3000:3000 \
  -v nexor-data:/data \
  -v $(pwd):/workspace \
  -e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY \
  ghcr.io/yourorg/nexor:latest
```

**Characteristics:**
- No account required
- All data stays local
- User provides their own API keys
- Free forever (open source core)

### Model 2: Self-Hosted with Account

Same as above, but user logs in for premium features.

```bash
docker run -d \
  --name nexor \
  -p 3000:3000 \
  -v nexor-data:/data \
  -v $(pwd):/workspace \
  -e NEXOR_LICENSE_KEY=nx_live_xxx \
  ghcr.io/yourorg/nexor:latest
```

**Unlocks:**
- Cloud sync (settings, history)
- Premium agent personas
- Priority support
- Team features (future)

### Model 3: Managed Cloud (Future)

We host nexor for users who don't want to manage Docker.

```
https://app.nexor.dev/workspace/my-project
```

**Characteristics:**
- No Docker required
- We manage infrastructure
- Subscription pricing
- Enterprise features (SSO, audit logs)

---

## Authentication & Accounts

### Local Auth (Default)

For self-hosted without account:

```
┌─────────────────────────────────────────────────────────────┐
│  First Run Setup                                            │
│                                                             │
│  Create a password for your local nexor instance:           │
│                                                             │
│  Password: ••••••••••••                                     │
│  Confirm:  ••••••••••••                                     │
│                                                             │
│  This password protects your nexor server.                  │
│  It's stored locally and never sent anywhere.               │
│                                                             │
│  [Create Password]                                          │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│  Or: [Sign in with nexor.dev account]                       │
└─────────────────────────────────────────────────────────────┘
```

**Flow:**
1. First run: User creates local password
2. Password hashed with Argon2, stored in SQLite
3. Login returns JWT stored in browser/TUI
4. No external calls, works offline

### Account Auth (Optional)

For users who want sync/premium:

```
┌─────────────────────────────────────────────────────────────┐
│  Sign in to nexor                                           │
│                                                             │
│  [Continue with GitHub]  ← Primary (dev tool)               │
│  [Continue with Google]                                     │
│  [Continue with Email]                                      │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│  Or: [Set up local-only password]                           │
└─────────────────────────────────────────────────────────────┘
```

**Flow:**
1. User clicks "Sign in with GitHub"
2. Browser opens `nexor.dev/auth/github`
3. OAuth flow completes, returns token
4. Token validated by nexor.dev API
5. Local instance stores account link

### Auth Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────────┐
│ nexor local │────▶│ nexor.dev   │────▶│ GitHub/Google   │
│  (Docker)   │     │  (our API)  │     │    OAuth        │
└─────────────┘     └─────────────┘     └─────────────────┘
       │                   │
       │                   ▼
       │            ┌─────────────┐
       │            │  Postgres   │
       │            │  (accounts) │
       │            └─────────────┘
       │
       ▼
┌─────────────┐
│   SQLite    │
│   (local)   │
└─────────────┘
```

### Token Structure

```rust
struct LocalToken {
    sub: String,           // "local" or account_id
    iat: i64,              // issued at
    exp: i64,              // expires
    instance_id: Uuid,     // unique per installation
}

struct AccountToken {
    sub: String,           // account_id
    email: String,
    plan: Plan,            // free, pro, team, enterprise
    features: Vec<Feature>,
    iat: i64,
    exp: i64,
}
```

---

## Data Architecture

### Local Storage (Docker Volume)

```
/data/
├── nexor.db              # SQLite database
├── config.toml           # User configuration
├── auth.json             # Encrypted auth state
├── .secret               # Instance secret key (for JWT signing)
└── cache/
    ├── models/           # LLM response cache (optional)
    └── github/           # GitHub API cache
```

### Database Schema Additions

```sql
-- New tables for server mode

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT,                    -- 'local' or account_id
    token_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_active TEXT NOT NULL,
    user_agent TEXT,
    ip_address TEXT
);

CREATE TABLE account_link (
    instance_id TEXT PRIMARY KEY,
    account_id TEXT,
    email TEXT,
    linked_at TEXT NOT NULL,
    last_sync TEXT,
    sync_enabled INTEGER DEFAULT 0
);

CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_used TEXT,
    scopes TEXT                      -- JSON array of permissions
);
```

### Cloud Sync (Optional)

When account linked and sync enabled:

```
Local SQLite ──sync──▶ nexor.dev ──sync──▶ Other devices
     │                     │
     │                     ▼
     │              ┌─────────────┐
     │              │  Postgres   │
     │              │  (per-user  │
     │              │   storage)  │
     │              └─────────────┘
     │
     ▼
Synced: settings, history, templates
NOT synced: code, credentials, full logs
```

---

## API Design

### REST Endpoints

```
Authentication
POST   /api/auth/local          # Local password login
POST   /api/auth/account        # Account token exchange
POST   /api/auth/logout         # Invalidate session
GET    /api/auth/me             # Current user info

Chat & Orchestration
POST   /api/chat                # Send message to orchestrator
GET    /api/chat/history        # Get conversation history
DELETE /api/chat/history        # Clear history

Tasks
GET    /api/tasks               # List all tasks
GET    /api/tasks/:id           # Get task details
POST   /api/tasks               # Create manual task
PATCH  /api/tasks/:id           # Update task
DELETE /api/tasks/:id           # Cancel task

Agents
GET    /api/agents              # List agent pool
GET    /api/agents/:id          # Agent details
POST   /api/agents/:id/stop     # Stop agent

Feed & Logs
GET    /api/feed                # Recent feed items
GET    /api/logs                # Detailed logs
GET    /api/logs/stream         # SSE log stream

Analytics
GET    /api/stats               # Usage statistics
GET    /api/costs               # Cost breakdown

Configuration
GET    /api/config              # Get current config
PATCH  /api/config              # Update config
GET    /api/config/models       # Available models

Workspace
GET    /api/files               # List workspace files
GET    /api/files/*path         # Read file
PUT    /api/files/*path         # Write file (with approval)
GET    /api/git/status          # Git status
POST   /api/git/commit          # Commit changes
```

### WebSocket Protocol

```
Connection: ws://localhost:3000/ws

Client → Server:
{
  "type": "subscribe",
  "channels": ["feed", "tasks", "agents"]
}

{
  "type": "chat",
  "message": "Add user authentication"
}

Server → Client:
{
  "type": "feed",
  "data": {
    "id": "...",
    "agent": "Worker 1",
    "content": "Looking at auth module...",
    "timestamp": "2026-01-27T10:30:00Z"
  }
}

{
  "type": "task_update",
  "data": {
    "id": "...",
    "status": "in_progress",
    "progress": 0.45
  }
}

{
  "type": "stream",
  "chat_id": "...",
  "delta": "I'll break this into"  // Streaming response
}
```

---

## Web UI

### Technology Choice

**Option A: Leptos (Rust WASM)** - Recommended
- Same language as backend
- Type-safe API calls
- Single build system
- Smaller bundle than React

**Option B: Static SPA (React/Svelte)**
- Faster development
- More developers available
- Separate build step
- Larger bundle

### Screens

```
/                     # Dashboard/Home
/chat                 # Main orchestrator chat
/feed                 # Agent activity feed
/tasks                # Task list and details
/agents               # Agent pool status
/costs                # Cost analytics
/files                # File browser/editor
/settings             # Configuration
/account              # Account management (if linked)
```

### Responsive Design

```
Desktop (>1200px)           Tablet (768-1200px)        Mobile (<768px)
┌────────┬─────────┐        ┌───────────────┐          ┌─────────┐
│ Sidebar│  Main   │        │    Main       │          │  Main   │
│        │ Content │        │   Content     │          │ Content │
│  Nav   │         │        ├───────────────┤          │         │
│        │         │        │  Bottom Nav   │          ├─────────┤
│        │         │        └───────────────┘          │ Bot Nav │
└────────┴─────────┘                                   └─────────┘
```

### Key UI Components

| Component | Purpose |
|-----------|---------|
| ChatInterface | Message input, streaming responses |
| FeedList | Real-time agent activity |
| TaskCard | Task status, progress, actions |
| AgentStatus | Pool visualization |
| FileTree | Browse workspace files |
| CodeEditor | Monaco-based file editing |
| CostChart | Token usage visualization |

---

## Docker Configuration

### Dockerfile

```dockerfile
# Build stage
FROM rust:1.75-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin nexor-server

# Web UI build (if using separate frontend)
FROM node:20-alpine AS ui-builder
WORKDIR /ui
COPY ui/ .
RUN npm ci && npm run build

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/nexor-server /app/
COPY --from=ui-builder /ui/dist /app/static

# Default port
EXPOSE 3000

# Data volume
VOLUME ["/data"]

# Workspace mount point
VOLUME ["/workspace"]

ENV NEXOR_DATA_DIR=/data
ENV NEXOR_WORKSPACE=/workspace
ENV NEXOR_HOST=0.0.0.0
ENV NEXOR_PORT=3000

ENTRYPOINT ["/app/nexor-server"]
```

### Docker Compose (Full Stack)

```yaml
# docker-compose.yml
version: '3.8'

services:
  nexor:
    image: ghcr.io/yourorg/nexor:latest
    container_name: nexor
    ports:
      - "3000:3000"
    volumes:
      - nexor-data:/data
      - .:/workspace:rw
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - GITHUB_TOKEN=${GITHUB_TOKEN}
      - NEXOR_LOG_LEVEL=info
    restart: unless-stopped

volumes:
  nexor-data:
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ANTHROPIC_API_KEY` | Yes* | - | Anthropic API key |
| `GITHUB_TOKEN` | No | - | GitHub personal access token |
| `NEXOR_PORT` | No | 3000 | HTTP server port |
| `NEXOR_HOST` | No | 0.0.0.0 | Bind address |
| `NEXOR_DATA_DIR` | No | /data | Data directory |
| `NEXOR_WORKSPACE` | No | /workspace | Workspace mount |
| `NEXOR_LOG_LEVEL` | No | info | Log verbosity |
| `NEXOR_LICENSE_KEY` | No | - | Premium license key |
| `NEXOR_ALLOW_REMOTE` | No | false | Allow non-localhost connections |

*Can be configured via Web UI after first run

---

## Distribution Strategy

### Channels

| Channel | Audience | Update Frequency |
|---------|----------|------------------|
| `latest` | Early adopters | On every release |
| `stable` | General users | Monthly |
| `lts` | Enterprise | Quarterly |

### Installation Methods

**1. Docker (Primary)**
```bash
# One-liner install
curl -fsSL https://nexor.dev/install.sh | sh

# Or direct Docker
docker pull ghcr.io/yourorg/nexor:latest
```

**2. Homebrew (macOS/Linux)**
```bash
brew install nexor
# Runs Docker under the hood or native binary
```

**3. Native Binary (Power Users)**
```bash
# Direct download
curl -L https://github.com/yourorg/nexor/releases/latest/download/nexor-$(uname -s)-$(uname -m) -o nexor
chmod +x nexor
```

### Auto-Updates

```
┌─────────────────────────────────────────────────────────────┐
│  Update Available                                           │
│                                                             │
│  nexor v1.2.0 → v1.3.0                                     │
│                                                             │
│  Changes:                                                   │
│  • New cost analytics dashboard                             │
│  • Improved agent coordination                              │
│  • Bug fixes                                                │
│                                                             │
│  [Update Now]  [Remind Later]  [Skip This Version]          │
│                                                             │
│  □ Auto-update in background (recommended)                  │
└─────────────────────────────────────────────────────────────┘
```

---

## Pricing & Licensing

### Tiers

| Tier | Price | Features |
|------|-------|----------|
| **Community** | Free | Full local functionality, no account needed |
| **Pro** | $19/mo | Cloud sync, premium personas, priority support |
| **Team** | $49/user/mo | Shared workspaces, team analytics, SSO |
| **Enterprise** | Custom | On-prem, audit logs, SLA, dedicated support |

### License Key Validation

```rust
struct LicenseKey {
    prefix: String,      // "nx_live_" or "nx_test_"
    account_id: String,
    plan: Plan,
    seats: Option<u32>,  // For team plans
    expires: Option<DateTime<Utc>>,
    signature: String,   // Ed25519 signature
}

// Validation flow
1. Parse and verify signature (offline capable)
2. Check expiration
3. Optionally phone home for revocation check
4. Cache validation for 24 hours
```

### Feature Flags

```toml
# What each tier unlocks
[features.community]
agents = true
tasks = true
github = true
file_editor = true
local_auth = true
max_workers = 6

[features.pro]
extends = "community"
cloud_sync = true
premium_personas = true
custom_models = true
max_workers = 12
priority_support = true

[features.team]
extends = "pro"
shared_workspaces = true
team_analytics = true
sso = true
audit_logs = true
max_workers = 24

[features.enterprise]
extends = "team"
on_premise = true
custom_deployment = true
dedicated_support = true
max_workers = "unlimited"
```

---

## Security Considerations

### Network Security

| Concern | Mitigation |
|---------|------------|
| Unauthorized access | Localhost-only by default, auth required |
| Man-in-the-middle | HTTPS optional for local, required for remote |
| API key exposure | Keys stored encrypted, never logged |
| XSS | CSP headers, sanitized output |
| CSRF | SameSite cookies, CSRF tokens |

### Container Security

```dockerfile
# Run as non-root
RUN useradd -r -u 1000 nexor
USER nexor

# Read-only filesystem where possible
# Minimal base image
# No shell in production image (optional)
```

### Secrets Management

```
User's API keys:
┌─────────────┐
│ Environment │──▶ nexor-server ──▶ LLM APIs
│  Variables  │         │
└─────────────┘         │
                        ▼
               ┌─────────────┐
               │  Encrypted  │
               │  in SQLite  │
               │ (at rest)   │
               └─────────────┘
```

### Workspace Isolation

```
Container has access to:
✓ /workspace (mounted codebase) - read/write
✓ /data (nexor state) - read/write
✗ Host filesystem - no access
✗ Host network - only mapped ports
✗ Other containers - isolated
```

---

## Migration Path

### From Current TUI to Server Mode

```
v1.x (Current)              v2.0 (Server Mode)
─────────────               ──────────────────
nexor binary    ──────────▶  nexor-server (Docker)
     │                            │
     ▼                            ▼
.nexor/state.db ──migrate──▶ /data/nexor.db
.nexor/config   ──migrate──▶ /data/config.toml
```

**Migration Script:**
```bash
# Automatic migration on first run
nexor migrate --from ~/.nexor --to /data

# Or manual
docker run -v ~/.nexor:/old -v nexor-data:/data \
  nexor:latest migrate
```

### Backward Compatibility

- TUI continues to work as client to server
- `nexor` command detects running server, connects to it
- Fallback to embedded mode if no server running

```bash
# Option 1: Embedded (current behavior)
nexor

# Option 2: Client mode (connects to server)
nexor --server http://localhost:3000

# Option 3: Server mode
nexor serve
# or
docker run ... nexor
```

---

## Implementation Phases

### Phase 1: Server Foundation (4-6 weeks)

| Task | Description |
|------|-------------|
| HTTP server setup | Axum server with basic routes |
| WebSocket infrastructure | Real-time connection handling |
| Local auth | Password-based local authentication |
| Session management | JWT creation, validation, refresh |
| Basic API endpoints | Chat, tasks, agents, config |
| Docker packaging | Dockerfile, compose, volumes |

**Deliverable:** Working server accessible via curl/Postman

### Phase 2: Web UI (4-6 weeks)

| Task | Description |
|------|-------------|
| UI framework setup | Leptos or React project |
| Authentication screens | Login, setup, account linking |
| Chat interface | Message input, streaming display |
| Feed view | Real-time activity updates |
| Task management | List, detail, actions |
| Responsive layout | Mobile-friendly design |

**Deliverable:** Full web interface parity with TUI

### Phase 3: Account System (3-4 weeks)

| Task | Description |
|------|-------------|
| nexor.dev backend | Account management API |
| OAuth integration | GitHub, Google providers |
| License key system | Generation, validation, features |
| Account linking | Local instance ↔ cloud account |
| Feature flags | Tier-based feature gating |

**Deliverable:** Working account system with free/pro tiers

### Phase 4: Cloud Sync (3-4 weeks)

| Task | Description |
|------|-------------|
| Sync protocol | Efficient delta sync |
| Conflict resolution | Last-write-wins or merge |
| Settings sync | Config, personas, templates |
| History sync | Conversation and task history |
| Selective sync | User controls what syncs |

**Deliverable:** Settings sync across devices

### Phase 5: Distribution & Polish (2-3 weeks)

| Task | Description |
|------|-------------|
| Auto-update system | Check, download, apply updates |
| Install scripts | One-liner installers |
| Documentation | User guide, API docs |
| Monitoring | Health checks, metrics |
| Error reporting | Optional crash reports |

**Deliverable:** Production-ready distribution

---

## Open Questions

### Technical

1. **Web UI framework?** Leptos (Rust) vs React/Svelte (JS)
   - Leptos: Single language, smaller bundle, newer ecosystem
   - React: Faster development, more libraries, separate build

2. **Sync granularity?** What exactly syncs to cloud?
   - Settings only? + History? + Task state?
   - Privacy implications of syncing work context

3. **Offline-first or online-first?**
   - Offline-first: Complex sync, better UX
   - Online-first: Simpler, requires connection for some features

### Business

4. **Open source strategy?**
   - Fully open source with paid hosting?
   - Open core with proprietary premium features?
   - Source available with commercial license?

5. **Pricing model?**
   - Flat subscription vs usage-based?
   - Free tier limitations?

6. **Enterprise requirements?**
   - On-premise deployment support?
   - Air-gapped environments?
   - Compliance certifications needed?

### Product

7. **TUI deprecation timeline?**
   - Maintain both indefinitely?
   - TUI becomes thin client only?
   - Sunset TUI after web UI matures?

8. **Mobile experience?**
   - Responsive web sufficient?
   - PWA with offline support?
   - Native apps ever?

---

## Success Metrics

| Metric | Target (6 months) |
|--------|-------------------|
| Docker pulls | 10,000+ |
| Active instances | 1,000+ |
| Accounts created | 500+ |
| Pro conversions | 5% of accounts |
| NPS score | 40+ |
| Uptime (nexor.dev) | 99.9% |

---

## Appendix: Alternative Approaches Considered

### A: Electron Desktop App
- **Pros:** Native feel, auto-updates, familiar distribution
- **Cons:** 100MB+ bundle, resource heavy, another runtime
- **Verdict:** Docker is lighter and more developer-friendly

### B: Pure CLI with Remote Backend
- **Pros:** Simplest local install
- **Cons:** Requires our hosting, no offline, ongoing costs
- **Verdict:** Doesn't serve self-hosted users

### C: Local Binary with Account Server
- **Pros:** No Docker required
- **Cons:** Complex cross-platform builds, update challenges
- **Verdict:** Docker solves these problems elegantly

---

## References

- [Docker Best Practices](https://docs.docker.com/develop/develop-images/dockerfile_best-practices/)
- [Axum Web Framework](https://github.com/tokio-rs/axum)
- [Leptos WASM Framework](https://leptos.dev/)
- [JWT Best Practices](https://datatracker.ietf.org/doc/html/rfc8725)
