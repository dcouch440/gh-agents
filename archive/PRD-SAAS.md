# nexor SaaS PRD

> Fully Hosted AI Agent Orchestration Platform

**Epic**: nexor Cloud - SaaS Platform
**Status**: Draft
**Author**: AI Assistant
**Date**: 2026-01-27

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Why SaaS](#why-saas)
3. [Product Vision](#product-vision)
4. [Architecture Overview](#architecture-overview)
5. [Multi-Tenancy Model](#multi-tenancy-model)
6. [User Journey](#user-journey)
7. [Workspace Management](#workspace-management)
8. [Agent Execution](#agent-execution)
9. [GitHub Integration](#github-integration)
10. [Billing & Pricing](#billing--pricing)
11. [Security & Compliance](#security--compliance)
12. [Infrastructure](#infrastructure)
13. [API & Integrations](#api--integrations)
14. [Competitive Analysis](#competitive-analysis)
15. [Go-to-Market](#go-to-market)
16. [Implementation Roadmap](#implementation-roadmap)
17. [Risks & Mitigations](#risks--mitigations)
18. [Success Metrics](#success-metrics)

---

## Executive Summary

**nexor Cloud** is a fully hosted SaaS platform where developers can:

1. **Connect their GitHub repos** - One-click OAuth integration
2. **Chat with AI orchestrator** - Natural language task assignment
3. **Watch agents work** - Real-time feed of agent activity
4. **Review & merge** - Approve PRs created by agents

No Docker. No local setup. No API key management. Just sign in and start.

```
Developer                    nexor Cloud                     GitHub
    │                            │                              │
    │  "Add authentication"      │                              │
    │ ─────────────────────────▶ │                              │
    │                            │  Clone repo                  │
    │                            │ ────────────────────────────▶│
    │                            │                              │
    │                            │  ◀─── repo contents ─────────│
    │                            │                              │
    │    ◀── Agent: "Breaking    │                              │
    │         into 4 slices..."  │                              │
    │                            │                              │
    │    ◀── Agent: "Working     │                              │
    │         on user model..."  │                              │
    │                            │                              │
    │    ◀── "PR ready for       │  Create PR                   │
    │         review"            │ ────────────────────────────▶│
    │                            │                              │
    │  Review & merge            │                              │
    │ ─────────────────────────▶ │  Merge PR                    │
    │                            │ ────────────────────────────▶│
```

---

## Why SaaS

### The Problem with Self-Hosted

| Barrier | Impact |
|---------|--------|
| Docker knowledge required | Excludes 60%+ of developers |
| API key management | Friction, security concerns |
| Local compute limits | Can't run many agents |
| No collaboration | Single-user only |
| Update friction | Users fall behind |

### SaaS Advantages

| Advantage | Benefit |
|-----------|---------|
| Zero setup | Sign in → working in 60 seconds |
| We manage LLM costs | Predictable pricing, no API key needed |
| Scalable compute | Run 10+ agents in parallel |
| Collaboration built-in | Teams share workspaces |
| Always updated | Latest features automatically |
| Mobile access | Work from anywhere |

### Target Users

**Primary**: Individual developers who want AI assistance without DevOps overhead

**Secondary**: Small teams (2-10) who want shared AI workflows

**Tertiary**: Enterprises wanting managed AI development tools

---

## Product Vision

### The 60-Second Experience

```
1. Land on nexor.dev                    (0:00)
2. Click "Sign in with GitHub"          (0:05)
3. Select repository                    (0:15)
4. Type "Add user authentication"       (0:30)
5. Watch agents start working           (0:45)
6. First status update appears          (0:60)
```

### Core Value Props

1. **Instant Start** - No setup, no config, no API keys
2. **Visible Progress** - Watch AI think and work in real-time
3. **Safe by Default** - Nothing merges without your approval
4. **Cost Predictable** - Fixed monthly price, unlimited tasks*

### Product Principles

- **GitHub-native** - Feels like an extension of GitHub
- **Transparent** - Never hide what agents are doing
- **Reversible** - Easy to undo any agent action
- **Progressive** - Simple start, power features available

---

## Architecture Overview

### High-Level System Design

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              nexor Cloud                                 │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                         Edge / CDN (Cloudflare)                    │ │
│  │                    nexor.dev, app.nexor.dev, api.nexor.dev         │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                    │                                     │
│  ┌─────────────────────────────────┼─────────────────────────────────┐  │
│  │                    Application Layer                               │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │  │
│  │  │   Web App    │  │   API        │  │  WebSocket   │             │  │
│  │  │   (Next.js)  │  │   (Rust)     │  │   Gateway    │             │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘             │  │
│  └─────────────────────────────────┼─────────────────────────────────┘  │
│                                    │                                     │
│  ┌─────────────────────────────────┼─────────────────────────────────┐  │
│  │                    Orchestration Layer                             │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │  │
│  │  │   Job Queue  │  │   Scheduler  │  │   Workspace  │             │  │
│  │  │   (Redis)    │  │              │  │   Manager    │             │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘             │  │
│  └─────────────────────────────────┼─────────────────────────────────┘  │
│                                    │                                     │
│  ┌─────────────────────────────────┼─────────────────────────────────┐  │
│  │                    Execution Layer                                 │  │
│  │  ┌────────────────────────────────────────────────────────────┐   │  │
│  │  │              Agent Pods (Kubernetes)                        │   │  │
│  │  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐       │   │  │
│  │  │  │ Agent 1 │  │ Agent 2 │  │ Agent 3 │  │   ...   │       │   │  │
│  │  │  │(sandbox)│  │(sandbox)│  │(sandbox)│  │         │       │   │  │
│  │  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘       │   │  │
│  │  └────────────────────────────────────────────────────────────┘   │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                    │                                     │
│  ┌─────────────────────────────────┼─────────────────────────────────┐  │
│  │                    Data Layer                                      │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐             │  │
│  │  │   Postgres   │  │     S3       │  │    Redis     │             │  │
│  │  │  (accounts,  │  │ (workspace   │  │   (cache,    │             │  │
│  │  │   billing)   │  │   storage)   │  │    queues)   │             │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘             │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
                │                                          │
                ▼                                          ▼
        ┌──────────────┐                          ┌──────────────┐
        │   Anthropic  │                          │    GitHub    │
        │     API      │                          │     API      │
        └──────────────┘                          └──────────────┘
```

### Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| **Frontend** | Next.js 14 + React | Fast, SEO, app router |
| **API** | Rust (Axum) | Performance, safety, existing code |
| **Real-time** | WebSockets + Redis pub/sub | Scalable broadcasting |
| **Queue** | Redis + BullMQ | Reliable job processing |
| **Database** | Postgres (Neon/Supabase) | Serverless, scalable |
| **Storage** | S3 / R2 | Workspace snapshots |
| **Compute** | Kubernetes (Fly.io/Railway) | Container orchestration |
| **CDN** | Cloudflare | Edge caching, DDoS protection |

---

## Multi-Tenancy Model

### Isolation Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│                        Shared Infrastructure                     │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  API Servers, WebSocket Gateway, Job Queue (multi-tenant) │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────┼───────────────────────────────┐  │
│  │           Per-User Isolation (Workspace Level)            │  │
│  │                                                            │  │
│  │  User A                    User B                          │  │
│  │  ┌──────────────────┐     ┌──────────────────┐            │  │
│  │  │ Workspace Pod    │     │ Workspace Pod    │            │  │
│  │  │ ┌──────────────┐ │     │ ┌──────────────┐ │            │  │
│  │  │ │ Cloned Repo  │ │     │ │ Cloned Repo  │ │            │  │
│  │  │ │ (ephemeral)  │ │     │ │ (ephemeral)  │ │            │  │
│  │  │ └──────────────┘ │     │ └──────────────┘ │            │  │
│  │  │ ┌──────────────┐ │     │ ┌──────────────┐ │            │  │
│  │  │ │ Agent procs  │ │     │ │ Agent procs  │ │            │  │
│  │  │ │ (sandboxed)  │ │     │ │ (sandboxed)  │ │            │  │
│  │  │ └──────────────┘ │     │ └──────────────┘ │            │  │
│  │  └──────────────────┘     └──────────────────┘            │  │
│  │                                                            │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │              Database (Row-Level Security)                  │  │
│  │  All queries filtered by user_id / org_id automatically    │  │
│  └────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Data Isolation

| Data Type | Isolation Method |
|-----------|------------------|
| User data | Row-level security (RLS) in Postgres |
| Workspace files | Separate S3 prefixes, signed URLs |
| Agent execution | Isolated Kubernetes pods |
| Secrets | Per-user encrypted vault |
| Logs | Filtered by user_id |

### Resource Limits

| Resource | Free | Pro | Team | Enterprise |
|----------|------|-----|------|------------|
| Concurrent agents | 2 | 6 | 12 | Custom |
| Workspace storage | 1 GB | 10 GB | 50 GB | Custom |
| Repos connected | 3 | 10 | Unlimited | Unlimited |
| Task history | 7 days | 90 days | 1 year | Custom |
| API requests/day | 100 | 1,000 | 10,000 | Custom |

---

## User Journey

### Signup Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│     ███╗   ██╗███████╗██╗  ██╗ ██████╗ ██████╗                 │
│     ████╗  ██║██╔════╝╚██╗██╔╝██╔═══██╗██╔══██╗                │
│     ██╔██╗ ██║█████╗   ╚███╔╝ ██║   ██║██████╔╝                │
│     ██║╚██╗██║██╔══╝   ██╔██╗ ██║   ██║██╔══██╗                │
│     ██║ ╚████║███████╗██╔╝ ██╗╚██████╔╝██║  ██║                │
│     ╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝                │
│                                                                 │
│              AI agents that ship code for you                   │
│                                                                 │
│            ┌─────────────────────────────────┐                  │
│            │   Continue with GitHub    🔗    │                  │
│            └─────────────────────────────────┘                  │
│                                                                 │
│                    No credit card required                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  GitHub OAuth                                                   │
│                                                                 │
│  nexor is requesting access to:                                 │
│                                                                 │
│  ✓ Read your profile information                                │
│  ✓ Read and write repository contents                           │
│  ✓ Create and manage pull requests                              │
│  ✓ Read organization membership                                 │
│                                                                 │
│  [Authorize nexor]                                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Select a repository to get started                             │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  🔍 Search repositories...                               │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Recent repositories:                                           │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  📁 acme/backend                              [Select]   │   │
│  │     TypeScript • Updated 2 hours ago                     │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  📁 acme/frontend                             [Select]   │   │
│  │     React • Updated yesterday                            │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  acme/backend                                    [Change repo]  │
│─────────────────────────────────────────────────────────────────│
│                                                                 │
│  👋 Welcome! I'm your AI orchestrator.                          │
│                                                                 │
│  I've analyzed your repository:                                 │
│  • TypeScript/Node.js backend                                   │
│  • Express.js with PostgreSQL                                   │
│  • Jest for testing                                             │
│                                                                 │
│  What would you like me to help you build?                      │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Try: "Add user authentication with JWT"                 │   │
│  │       "Fix the bug in issue #42"                         │   │
│  │       "Add rate limiting to the API"                     │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  > _                                                     │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Main Workspace View

```
┌─────────────────────────────────────────────────────────────────────────┐
│  nexor                 acme/backend ▼        Pro Plan    david ▼       │
├─────────┬───────────────────────────────────────────────────────────────┤
│         │                                                               │
│  Chat   │  You: Add user authentication with JWT and refresh tokens    │
│  ───    │                                                               │
│  Feed   │  Orchestrator: I'll break this into 4 vertical slices:       │
│         │                                                               │
│  Tasks  │    1. User model + database migration                         │
│         │    2. Register & login endpoints                              │
│  Files  │    3. JWT middleware + refresh token flow                     │
│         │    4. Protected route examples + tests                        │
│  PRs    │                                                               │
│         │  Starting work on Slice 1...                                  │
│  Costs  │                                                               │
│         │  ┌─────────────────────────────────────────────────────────┐ │
│  ─────  │  │ Worker 1: Creating user migration...                    │ │
│         │  │ ████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░ 30%            │ │
│  Settings  └─────────────────────────────────────────────────────────┘ │
│         │                                                               │
│         │  ┌───────────────────────────────────────────────────────┐   │
│         │  │ > _                                                    │   │
│         │  └───────────────────────────────────────────────────────┘   │
└─────────┴───────────────────────────────────────────────────────────────┘
```

### Agent Feed View

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Feed                                              Live ● 3 agents      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  10:42:15  ● Orchestrator                                               │
│            Breaking down "Add user authentication" into slices:         │
│            → Slice 1: User model + migration                            │
│            → Slice 2: Auth endpoints                                    │
│            → Slice 3: JWT middleware                                    │
│            → Slice 4: Tests                                             │
│                                                                         │
│  10:42:18  ● Worker 1 assigned to Slice 1                               │
│            Starting work on user model...                               │
│                                                                         │
│  10:42:23  ● Worker 1                                                   │
│            Found existing Prisma schema at prisma/schema.prisma         │
│            Adding User model with email, passwordHash, createdAt        │
│                                                                         │
│  10:42:31  ● Worker 1                                                   │
│            Created migration: 20260127_add_user_model                   │
│            Running migration... ✓                                       │
│                                                                         │
│  10:42:35  ★ MILESTONE: Slice 1 complete                                │
│            User model ready, migration applied                          │
│                                                                         │
│  10:42:36  ● Worker 2 assigned to Slice 2                               │
│            Starting auth endpoints...                                   │
│                                                                         │
│  10:42:38  ● Worker 1 assigned to Slice 3                               │
│            Starting JWT middleware...                                   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Workspace Management

### Workspace Lifecycle

```
User selects repo
       │
       ▼
┌──────────────┐
│ Provisioning │  Clone repo, analyze structure, warm cache
└──────────────┘
       │
       ▼
┌──────────────┐
│    Active    │  Agents working, real-time updates
└──────────────┘
       │
       ├─── User idle 30min ───▶ ┌──────────────┐
       │                         │   Sleeping   │  Pods scaled down
       │                         └──────────────┘
       │                                │
       │    ◀─── User returns ──────────┘
       │
       ├─── Task complete ───▶ ┌──────────────┐
       │                       │   Idle       │  Minimal resources
       │                       └──────────────┘
       │
       └─── User disconnects ───▶ ┌──────────────┐
              for 24h+            │  Archived    │  Saved to S3
                                  └──────────────┘
```

### Workspace Storage

```
S3 Bucket: nexor-workspaces
└── user_abc123/
    └── workspace_xyz789/
        ├── repo.tar.gz           # Compressed repo snapshot
        ├── state.json            # Agent state, task history
        ├── .nexor/
        │   ├── config.toml       # User preferences
        │   └── cache/            # LLM response cache
        └── snapshots/
            ├── 2026-01-27T10:00.tar.gz
            └── 2026-01-27T11:00.tar.gz
```

### Branch Strategy

```
main (protected)
  │
  ├── nexor/task-123-user-auth
  │     └── Agents work here
  │
  └── nexor/task-124-rate-limiting
        └── Separate branch per task

PR: nexor/task-123-user-auth → main
    Created automatically when task completes
    User reviews and merges
```

---

## Agent Execution

### Pod Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Workspace Pod (per active workspace)                           │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Init Container                                            │ │
│  │  • Clone repo from GitHub                                  │ │
│  │  • Restore workspace state from S3                         │ │
│  │  • Install dependencies (cached)                           │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Orchestrator Container                                    │ │
│  │  • Single orchestrator agent                               │ │
│  │  • Manages task queue                                      │ │
│  │  • Coordinates workers                                     │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │  Worker 1   │  │  Worker 2   │  │  Worker N   │            │
│  │  Container  │  │  Container  │  │  Container  │            │
│  │             │  │             │  │             │            │
│  │  • Sandboxed│  │  • Sandboxed│  │  • Sandboxed│            │
│  │  • File R/W │  │  • File R/W │  │  • File R/W │            │
│  │  • No net*  │  │  • No net*  │  │  • No net*  │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  Shared Volume: /workspace                                 │ │
│  │  • Cloned repository                                       │ │
│  │  • Read/write for all containers                           │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│  *Network allowed only for: GitHub API, package registries     │
└─────────────────────────────────────────────────────────────────┘
```

### Resource Allocation

| Tier | CPU | Memory | Workers | Monthly Cost (to us) |
|------|-----|--------|---------|---------------------|
| Free | 0.5 | 512 MB | 2 | ~$5 |
| Pro | 2 | 2 GB | 6 | ~$20 |
| Team | 4 | 4 GB | 12 | ~$40 |
| Enterprise | 8+ | 8+ GB | Custom | ~$100+ |

### Execution Safety

| Risk | Mitigation |
|------|------------|
| Malicious code | Sandboxed containers, no host access |
| Infinite loops | Timeout limits (5 min per operation) |
| Resource exhaustion | CPU/memory limits, pod eviction |
| Network abuse | Egress restricted to allowlist |
| Data exfiltration | No arbitrary network access |
| Secrets exposure | Encrypted vault, scoped access |

---

## GitHub Integration

### OAuth Scopes Required

```
repo                 # Full access to repositories
read:user            # Read user profile
user:email           # Read user email
read:org             # Read organization membership
```

### Webhook Events

| Event | Action |
|-------|--------|
| `push` | Sync workspace with latest changes |
| `pull_request` | Track PR status for agent-created PRs |
| `issue_comment` | Trigger agent on @nexor mentions |
| `issues` | Import issues as potential tasks |

### PR Workflow

```
Agent completes task
       │
       ▼
┌──────────────────┐
│ Create PR draft  │  nexor/task-123 → main
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Run CI checks    │  User's existing CI runs
└──────────────────┘
       │
       ▼
┌──────────────────┐
│ Notify user      │  In-app + email notification
└──────────────────┘
       │
       ▼
User reviews in GitHub or nexor
       │
       ├─── Approve ───▶ ┌──────────────┐
       │                 │ Merge PR     │
       │                 └──────────────┘
       │
       └─── Request changes ───▶ ┌──────────────┐
                                 │ Agent revises│
                                 └──────────────┘
```

### Issue Integration

```
┌─────────────────────────────────────────────────────────────────┐
│  Import from GitHub Issues                                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  □ #42  Fix login redirect bug                     [Import]     │
│         bug, priority:high                                      │
│                                                                 │
│  □ #43  Add password reset flow                    [Import]     │
│         enhancement                                             │
│                                                                 │
│  ☑ #44  Update dependencies                        [Imported]   │
│         chore • Assigned to nexor                               │
│                                                                 │
│  [Import Selected]  [Auto-import labeled issues]                │
└─────────────────────────────────────────────────────────────────┘
```

---

## Billing & Pricing

### Pricing Tiers

| Tier | Monthly | Annual | Includes |
|------|---------|--------|----------|
| **Free** | $0 | $0 | 50 tasks/mo, 2 agents, 3 repos |
| **Pro** | $29 | $290 ($24/mo) | Unlimited tasks, 6 agents, 10 repos |
| **Team** | $49/user | $490/user | + Shared workspaces, 12 agents, SSO |
| **Enterprise** | Custom | Custom | + On-prem, SLA, dedicated support |

### What's a "Task"?

A task = one unit of work that produces a PR or resolves an issue.

Examples:
- "Add user authentication" = 1 task (even if 4 slices)
- "Fix bug #42" = 1 task
- "What's in this file?" = 0 tasks (conversation only)

### LLM Cost Pass-Through

We absorb LLM costs in the subscription. Rough economics:

| Task Complexity | Avg LLM Cost | Tasks at $29/mo |
|-----------------|--------------|-----------------|
| Simple (1 slice) | $0.50 | 58 tasks |
| Medium (3 slices) | $1.50 | 19 tasks |
| Complex (5 slices) | $3.00 | 9 tasks |

**Unlimited** means fair use (~100 tasks/month on Pro). Abusers contacted.

### Billing Events

```sql
-- Billing events table
CREATE TABLE billing_events (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    event_type TEXT NOT NULL,  -- 'task_completed', 'overage', etc.
    amount_cents INTEGER,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Usage tracking
CREATE TABLE usage (
    user_id UUID PRIMARY KEY,
    period_start DATE NOT NULL,
    tasks_used INTEGER DEFAULT 0,
    tasks_limit INTEGER NOT NULL,
    compute_minutes INTEGER DEFAULT 0,
    storage_bytes BIGINT DEFAULT 0
);
```

### Payment Integration

- **Provider**: Stripe
- **Features**: Subscriptions, usage-based billing, invoicing
- **Trial**: 14-day Pro trial, no credit card required

---

## Security & Compliance

### Security Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Security Layers                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Edge Security (Cloudflare)                                  │
│     • DDoS protection                                           │
│     • WAF rules                                                 │
│     • Rate limiting                                             │
│     • Bot detection                                             │
│                                                                 │
│  2. Authentication                                              │
│     • GitHub OAuth (primary)                                    │
│     • JWT with short expiry (1h)                                │
│     • Refresh tokens (encrypted)                                │
│     • Session management                                        │
│                                                                 │
│  3. Authorization                                               │
│     • Row-level security in Postgres                            │
│     • Resource-based access control                             │
│     • Org membership validation                                 │
│                                                                 │
│  4. Data Protection                                             │
│     • Encryption at rest (AES-256)                              │
│     • Encryption in transit (TLS 1.3)                           │
│     • Secrets in HashiCorp Vault                                │
│     • PII minimization                                          │
│                                                                 │
│  5. Execution Isolation                                         │
│     • Kubernetes network policies                               │
│     • Seccomp profiles                                          │
│     • Read-only root filesystem                                 │
│     • Non-root containers                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Compliance Roadmap

| Standard | Target Date | Status |
|----------|-------------|--------|
| SOC 2 Type I | Q3 2026 | Planned |
| SOC 2 Type II | Q1 2027 | Planned |
| GDPR | Launch | Required |
| HIPAA | Q4 2027 | If demand |

### Data Retention

| Data Type | Retention | Deletion |
|-----------|-----------|----------|
| Account info | While active | On request |
| Task history | Per plan (7d-1y) | Auto-purge |
| Workspace files | 30 days inactive | Auto-purge |
| Logs | 90 days | Auto-purge |
| Billing records | 7 years | Legal requirement |

---

## Infrastructure

### Cloud Provider Strategy

**Primary**: Fly.io or Railway
- Edge deployment
- Simple Kubernetes alternative
- Reasonable pricing

**Alternative**: AWS/GCP
- For enterprise customers
- Region-specific requirements

### Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Compute | Fly.io Machines | Fast spinup, edge locations |
| Database | Neon Postgres | Serverless, branching |
| Cache | Upstash Redis | Serverless, global |
| Storage | Cloudflare R2 | S3-compatible, no egress |
| CDN | Cloudflare | Free tier, global |
| Monitoring | Axiom | Log aggregation |
| Error tracking | Sentry | Exception tracking |

### Scaling Strategy

```
            Users
              │
              ▼
┌─────────────────────────┐
│    Load Balancer        │  Fly.io anycast
└─────────────────────────┘
              │
     ┌────────┼────────┐
     ▼        ▼        ▼
┌─────────┐ ┌─────────┐ ┌─────────┐
│ API Pod │ │ API Pod │ │ API Pod │   Auto-scale 2-10
└─────────┘ └─────────┘ └─────────┘
     │        │        │
     └────────┼────────┘
              ▼
┌─────────────────────────┐
│    Job Queue (Redis)    │  Upstash, serverless
└─────────────────────────┘
              │
     ┌────────┼────────┐
     ▼        ▼        ▼
┌─────────┐ ┌─────────┐ ┌─────────┐
│Workspace│ │Workspace│ │Workspace│   Scale per demand
│  Pod    │ │  Pod    │ │  Pod    │   Sleep when idle
└─────────┘ └─────────┘ └─────────┘
```

### Cost Projections

| Users | Monthly Infra Cost | Revenue (avg $20/user) | Margin |
|-------|-------------------|------------------------|--------|
| 100 | $500 | $2,000 | 75% |
| 1,000 | $3,000 | $20,000 | 85% |
| 10,000 | $20,000 | $200,000 | 90% |

---

## API & Integrations

### Public API

```
Base URL: https://api.nexor.dev/v1

Authentication: Bearer token (API key)

Endpoints:
POST   /tasks              Create a task programmatically
GET    /tasks/:id          Get task status
GET    /tasks/:id/feed     Stream task updates (SSE)
POST   /tasks/:id/cancel   Cancel a running task
GET    /repos              List connected repositories
POST   /repos/:id/sync     Trigger repo sync
GET    /usage              Get usage statistics
```

### CLI Tool

```bash
# Install
npm install -g @nexor/cli
# or
brew install nexor

# Login
nexor login

# Run task from terminal
nexor run "Add user authentication" --repo acme/backend

# Watch progress
nexor watch task_abc123

# List recent tasks
nexor tasks
```

### IDE Extensions

```
┌─────────────────────────────────────────────────────────────────┐
│  VS Code Extension                                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  NEXOR                                          [Connected ●]   │
│                                                                 │
│  Active Tasks:                                                  │
│  ├── #123 Add authentication        [In Progress ███░░░]       │
│  └── #124 Fix login bug             [Completed ✓]              │
│                                                                 │
│  Quick Actions:                                                 │
│  [New Task]  [View Feed]  [Open Dashboard]                      │
│                                                                 │
│  Recent:                                                        │
│  • PR #42 ready for review                                      │
│  • Task #123 created 3 new files                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Webhooks (Outgoing)

```json
// Task completed webhook
POST https://your-server.com/webhooks/nexor
{
  "event": "task.completed",
  "task_id": "task_abc123",
  "repo": "acme/backend",
  "pr_url": "https://github.com/acme/backend/pull/42",
  "summary": "Added user authentication with JWT",
  "timestamp": "2026-01-27T10:45:00Z"
}
```

---

## Competitive Analysis

### Market Landscape

| Product | Type | Strengths | Weaknesses |
|---------|------|-----------|------------|
| **GitHub Copilot** | Autocomplete | Ubiquitous, inline | Not agentic, single-file |
| **Cursor** | IDE | Good UX, fast | Local only, manual |
| **Devin** | Agent | Full autonomy | Expensive, opaque |
| **Replit Agent** | Agent | In-browser | Limited to Replit |
| **Codeium** | Autocomplete | Free tier | Less capable |

### nexor Positioning

```
                    Autonomous
                        │
              Devin ●   │
                        │
                        │   ● nexor
                        │
                        │
    Simple ─────────────┼───────────────── Complex
                        │
           Copilot ●    │    ● Cursor
                        │
                        │
                    Assisted
```

**Our Niche**: Autonomous enough to ship PRs, transparent enough to trust.

### Differentiation

| Feature | Copilot | Cursor | Devin | nexor |
|---------|---------|--------|-------|-------|
| Zero setup | ✓ | ✗ | ✓ | ✓ |
| Creates PRs | ✗ | ✗ | ✓ | ✓ |
| Transparent progress | - | - | ✗ | ✓ |
| Vertical slicing | ✗ | ✗ | ? | ✓ |
| Cost visibility | N/A | N/A | ✗ | ✓ |
| Works offline | ✗ | ✓ | ✗ | ✗ |

---

## Go-to-Market

### Launch Strategy

**Phase 1: Private Beta** (2 months)
- 100 hand-picked users
- High-touch onboarding
- Daily feedback collection
- Iterate rapidly

**Phase 2: Public Beta** (2 months)
- Open signups with waitlist
- Free tier only
- Focus on virality (share task results)
- Content marketing (demos, tutorials)

**Phase 3: General Availability**
- Paid tiers launch
- Remove waitlist
- PR push (TechCrunch, Hacker News)
- Affiliate/referral program

### Marketing Channels

| Channel | Strategy | Goal |
|---------|----------|------|
| **Twitter/X** | Demo videos, founder presence | Awareness |
| **Hacker News** | Show HN posts, engage comments | Early adopters |
| **Dev.to/Medium** | Technical tutorials | SEO, credibility |
| **YouTube** | "Watch AI build X" series | Virality |
| **Discord** | Community, support | Retention |
| **Referrals** | $10 credit per referral | Growth |

### Metrics to Track

| Metric | Target (6 months) |
|--------|-------------------|
| Signups | 10,000 |
| WAU | 2,000 |
| Tasks completed | 50,000 |
| PRs merged | 10,000 |
| Paid conversions | 5% |
| NPS | 50+ |
| Churn (monthly) | <5% |

---

## Implementation Roadmap

### Phase 1: Core Platform (8 weeks)

| Week | Focus | Deliverables |
|------|-------|--------------|
| 1-2 | Infrastructure | Fly.io setup, Postgres, Redis, S3 |
| 3-4 | Auth & accounts | GitHub OAuth, user management, API keys |
| 5-6 | Workspace management | Clone, provision, lifecycle |
| 7-8 | Agent execution | Pod orchestration, sandboxing |

**Milestone**: Can run a single agent on a repo

### Phase 2: Web UI (6 weeks)

| Week | Focus | Deliverables |
|------|-------|--------------|
| 9-10 | Foundation | Next.js setup, auth flow, layout |
| 11-12 | Chat & feed | Real-time updates, streaming |
| 13-14 | Task management | Task list, details, actions |

**Milestone**: Functional web UI for private beta

### Phase 3: GitHub Integration (4 weeks)

| Week | Focus | Deliverables |
|------|-------|--------------|
| 15-16 | PR workflow | Create, track, merge PRs |
| 17-18 | Webhooks | Push, PR events, issue sync |

**Milestone**: Full GitHub round-trip working

### Phase 4: Billing & Polish (4 weeks)

| Week | Focus | Deliverables |
|------|-------|--------------|
| 19-20 | Stripe integration | Plans, checkout, portal |
| 21-22 | Polish | Error handling, edge cases, docs |

**Milestone**: Ready for public beta

### Phase 5: Scale & Enterprise (Ongoing)

- Team features
- SSO integration
- On-premise option
- SOC 2 compliance

---

## Risks & Mitigations

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| LLM costs spike | Medium | High | Usage limits, monitoring, model tiering |
| Agent produces bad code | High | Medium | PR review required, easy rollback |
| Security breach | Low | Critical | Isolation, audits, bug bounty |
| GitHub rate limits | Medium | Medium | Caching, webhook-first architecture |
| Scaling issues | Medium | High | Load testing, auto-scaling |

### Business Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Competitor launches | High | Medium | Move fast, differentiate on UX |
| LLM provider issues | Low | High | Multi-provider support |
| Low conversion | Medium | High | Focus on value, pricing experiments |
| Support overhead | Medium | Medium | Self-serve docs, community |

### Regulatory Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| GDPR compliance | Certain | High | Privacy by design, DPA ready |
| AI regulation | Medium | Unknown | Monitor, adaptable architecture |
| IP concerns | Low | Medium | Clear ToS, user owns output |

---

## Success Metrics

### North Star Metrics

1. **PRs Merged** - Ultimate value delivery
2. **Weekly Active Users** - Engagement
3. **Net Revenue Retention** - Business health

### Dashboard

```
┌─────────────────────────────────────────────────────────────────┐
│  nexor Metrics Dashboard                         January 2026   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Users          Tasks           PRs Merged        Revenue       │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐      ┌─────────┐   │
│  │  2,847  │    │ 12,453  │    │  3,891  │      │ $52.3K  │   │
│  │  +23%   │    │  +45%   │    │  +38%   │      │  +31%   │   │
│  └─────────┘    └─────────┘    └─────────┘      └─────────┘   │
│                                                                 │
│  Conversion Funnel                    LLM Cost per Task         │
│  Signup ████████████████ 10,000       $1.24 avg (target: <$1.50)│
│  Activated ██████████░░░ 6,500                                  │
│  Task Created ███████░░░ 4,200        Agent Success Rate        │
│  PR Merged ████░░░░░░░░░ 2,100        87% (target: >85%)        │
│  Paid ██░░░░░░░░░░░░░░░░   520                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Appendix: Comparison with Docker Self-Hosted

| Aspect | Self-Hosted (Docker) | SaaS |
|--------|---------------------|------|
| Setup time | 5-10 minutes | 60 seconds |
| Maintenance | User responsibility | We handle it |
| Updates | Manual or auto-pull | Automatic |
| Data location | User's machine | Our cloud |
| API keys | User provides | We provide |
| Scaling | Limited by machine | Elastic |
| Collaboration | Single user | Multi-user |
| Offline | Yes | No |
| Privacy | Maximum | Trust us |
| Cost | Free + API costs | Subscription |

### Recommendation

**Offer both:**
1. **SaaS** (primary) - For most users, easiest path
2. **Self-hosted** - For privacy-conscious, enterprises, air-gapped

The self-hosted option also serves as a hedge against cloud skepticism and a way to build trust ("you can always run it yourself").

---

## Next Steps

1. **Validate assumptions** with potential users
2. **Prototype** workspace provisioning (hardest technical piece)
3. **Design** detailed UI mockups
4. **Estimate** infrastructure costs more precisely
5. **Define** MVP scope for private beta
