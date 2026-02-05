# Milestone 16: SaaS Foundation

> Refactor nexor from a local single-user app to a cloud-hosted multi-tenant SaaS platform at nexor.io. Real user accounts, GitHub OAuth, Postgres, cloud-hosted repos, collaborative chat rooms with AI participation, and an onboarding flow that gets developers from sign-up to working in under 2 minutes.

## Overview

This milestone is the architectural shift from local tool to hosted platform. The end state: a developer downloads the app (or hits nexor.io), signs up with GitHub, connects their repos, invites teammates, and starts working — editing code, managing prompts, chatting with teammates while an AI listens and contributes.

Core changes:

1. **Postgres Migration** — Replace SQLite with Postgres for multi-tenant persistence
2. **User Accounts & Orgs** — Real user model with organizations, teams, roles, invitations
3. **GitHub OAuth & Connect** — Sign in with GitHub, connect account, browse and import repos
4. **Cloud Repo Management** — Server-side repo clones per org, sandboxed storage, sync via GitHub
5. **Collaborative Chat Rooms** — Shared async chat where team members talk and the AI listens, learns context, and contributes periodically
6. **Presence & Awareness** — Who's online, who's viewing what, typing indicators
7. **Multi-Tenant Isolation** — Every query scoped to org, tenant-aware middleware
8. **Encrypted Secrets** — Per-tenant API key and token storage
9. **Onboarding Wizard** — Sign up → GitHub connect → pick repos → invite team → start working

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                       nexor.io (React)                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────────────┐   │
│  │  OAuth   │ │ Onboard  │ │  Collab   │ │   Workspace       │   │
│  │  Login   │ │  Wizard  │ │  Chat     │ │   (M15 features)  │   │
│  └──────────┘ └──────────┘ └──────────┘ └───────────────────┘   │
│  ┌──────────────────────────────────────────────────────────────┐│
│  │  Presence indicators, user avatars, online status            ││
│  └──────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
                         │ HTTP + WebSocket
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Axum Server (multi-tenant)                     │
│  ┌─────────┐ ┌──────────┐ ┌───────────┐ ┌────────────────────┐  │
│  │  Auth   │ │  Tenant  │ │  GitHub   │ │  Chat/Presence     │  │
│  │  OAuth  │ │  Middleware│ │  API      │ │  Broadcast         │  │
│  └─────────┘ └──────────┘ └───────────┘ └────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
          │                    │                    │
          ▼                    ▼                    ▼
┌──────────────┐  ┌─────────────────┐  ┌────────────────────────┐
│   Postgres   │  │  Repo Storage   │  │  Orchestration Core    │
│  (multi-     │  │  (per-org       │  │  (agents, LLM,         │
│   tenant)    │  │   sandboxed)    │  │   execution)           │
└──────────────┘  └─────────────────┘  └────────────────────────┘
```

## Dependencies

- Requires M10 (Server Layer) — complete
- Requires M11 (React Foundation) — in progress
- M15 features (editor, prompts, reports) layer on top of M16 tenant model
- GitHub OAuth requires registering a GitHub OAuth App

---

## Tickets

| Ticket | Title | Slices | Priority |
|--------|-------|--------|----------|
| 16.1 | Postgres Migration | 6 | P0 |
| 16.2 | User Accounts & Org Model | 7 | P0 |
| 16.3 | GitHub OAuth & Account Connect | 6 | P0 |
| 16.4 | Cloud Repo Management | 7 | P0 |
| 16.5 | Multi-Tenant Data Isolation | 5 | P0 |
| 16.6 | Encrypted Secrets Storage | 4 | P0 |
| 16.7 | Collaborative Chat Rooms | 8 | P1 |
| 16.8 | Presence & User Awareness | 5 | P1 |
| 16.9 | Onboarding Wizard | 6 | P1 |

---

## Ticket Summaries

### 16.1: Postgres Migration
Replace SQLite with Postgres. Rewrite all migrations, update connection pooling (deadpool-postgres or sqlx), update all DB queries, add connection config for DATABASE_URL.

### 16.2: User Accounts & Org Model
Real user model with email, avatar, display name. Organizations with memberships and roles (owner, admin, member, viewer). Invitation system with email or link. First user creates an org automatically.

### 16.3: GitHub OAuth & Account Connect
GitHub OAuth app integration for sign-in. Account connection page where users authorize nexor to access their GitHub. Token storage for GitHub API access. Browse user's repos and orgs from their GitHub account.

### 16.4: Cloud Repo Management
Server-side repo clones in per-org sandboxed storage directories. Clone from GitHub on import, periodic sync/pull. Disk quota per org. Replace local path references with server-managed paths. Invitation to a repo = granting access to that repo within the org.

### 16.5: Multi-Tenant Data Isolation
Add `org_id` to every tenant-scoped table. Tenant-aware middleware extracts org from auth token and injects into all queries. Verify no cross-tenant data leaks. Shared tables (system prompts, plans) remain global.

### 16.6: Encrypted Secrets Storage
Per-tenant encrypted storage for API keys (Anthropic, GitHub tokens). Encryption at rest using AES-256-GCM with a server master key. API to set/rotate/delete secrets. Never return raw secrets in API responses.

### 16.7: Collaborative Chat Rooms
Shared async chat rooms per org (or per repo). Multiple users and AI in the same room. Messages stored with user attribution and timestamps. AI listens to conversation context and contributes periodically — status updates ("just finished the auth PR"), relevant observations ("Mark makes a great point about the schema"), answers to questions directed at it. WebSocket broadcast to all room participants.

### 16.8: Presence & User Awareness
Real-time presence system: who's online, what file/page they're viewing, typing indicators in chat rooms. User avatars (pulled from GitHub) shown throughout the UI. "Currently viewing" indicators on files in the editor.

### 16.9: Onboarding Wizard
Step-by-step flow: Sign up (or GitHub login) → Connect GitHub account → Browse and select repos to import → Invite teammates (email or link) → Land on first project workspace. Progress bar, skip options, can complete later.

---

## Key Design Decisions

### SQLite → Postgres
Postgres is required for concurrent multi-user writes, proper transactions under load, and horizontal scaling later. Use `sqlx` with compile-time query checking for type safety.

### Chat Room AI Behavior
The AI is a **participant**, not just a responder. It:
- Listens to all messages in the room
- Proactively posts updates about agent work ("PR #42 is ready for review")
- Responds when addressed ("@nexor what's the status of the auth refactor?")
- Occasionally contributes relevant context ("Mark makes a great point — that approach aligns with the pattern in src/auth/")
- Respects a cooldown so it doesn't flood the chat (configurable per room)
- Can be muted by any user

### Repo Access Model
Like VS Code Live Share — someone in the org imports a repo, then other org members can see and work on it. Permissions are org-level (all members see all org repos) initially. Per-repo permissions are a future enhancement.

### Onboarding Philosophy
Under 2 minutes from landing page to working. GitHub OAuth eliminates manual registration. Repo import is one click per repo. The wizard is skippable at every step — power users can set up later.

---

*Created: 2026-01-29*
