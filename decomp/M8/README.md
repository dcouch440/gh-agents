# Milestone 8: GitHub Integration

> Pull issues from GitHub, have agents work them, create PRs.

## Goal

Build GitHub integration that allows nexor to:
- Authenticate with GitHub API
- Fetch issues from repositories
- Convert issues to internal tickets for agent processing
- Create pull requests from completed work
- Post progress updates back to issues

**Checkpoint**: Fetch a GitHub issue, have agents work it, create a PR.

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 8.0 | GitHub Authentication | 5 | M1 (Foundation) |
| 8.1 | GitHub API Client | 4 | 8.0 |
| 8.2 | Issue Sync | 3 | 8.1 |
| 8.3 | PR Creation | 3 | 8.1, M7 (Execution Layer) |
| 8.4 | Progress Updates | 3 | 8.1 |
| 8.5 | PR Retrieval & Review | 4 | 8.1 |
| 8.6 | PR Merge Operations | 4 | 8.1, 8.5 |
| 8.7 | PR Merge Queue & Conflict Resolution | 6 | 7.6, 8.5, 8.6, 8.4 |

**Total Slices**: 32

---

## Dependency Graph

```
M1 (Foundation)
    │
    ▼
   8.0 GitHub Authentication  ←── Device Flow login
    │
    ▼
   8.1 GitHub API Client
    │
    ├────────────┬────────────┬────────────┐
    ▼            ▼            ▼            ▼
   8.2          8.3          8.4          8.5
  Issue         PR         Progress    PR Retrieval
   Sync       Creation     Updates     & Review
              │                         │
              ▼                         ▼
         M7 (Execution)                8.6
              │                      PR Merge
              │                      Operations
              │                         │
              └─────────┬───────────────┘
                        │
                        ▼
                       8.7
                  PR Merge Queue
                  & Conflict Resolution
                        │
                        ▼
                  7.6 (Git Merge Operations)
```

---

## Parallelization

**Must be sequential**:
- 8.0 → 8.1 (API client needs auth/token)
- 8.5 → 8.6 (merge operations need retrieval types)
- 8.5, 8.6, 7.6 → 8.7 (merge queue needs all merge infrastructure)

**Can run in parallel** (after 8.1 is complete):
- 8.2 Issue Sync
- 8.4 Progress Updates
- 8.5 PR Retrieval & Review

**Must wait for M7**:
- 8.3 PR Creation (needs git operations from M7)
- 8.7 PR Merge Queue (needs 7.6 Git Merge Operations)

**Recommended execution order**:
1. 8.0 GitHub Authentication
2. 8.1 GitHub API Client
3. 8.2, 8.4, 8.5 in parallel
4. 8.6 PR Merge Operations (after 8.5)
5. 8.3 PR Creation (after M7)
6. 8.7 PR Merge Queue (after 7.6, 8.5, 8.6, 8.4)

---

## File Structure

All GitHub integration code goes in `src/github/`:

```
src/github/
├── mod.rs              ← Public exports, GitHub module root
├── auth.rs             ← Device Flow authentication (8.0)
├── client.rs           ← GitHub API HTTP client
├── types.rs            ← GitHub-specific types (Issue, PR, etc.)
├── issue_sync.rs       ← Issue to Ticket conversion
├── pr.rs               ← Pull request creation, retrieval, review, merge
├── comments.rs         ← Issue commenting
└── merge_queue.rs      ← PR merge queue management and conflict resolution
```

Credentials storage in `src/config/`:

```
src/config/
├── credentials.rs      ← Secure token storage
└── ...
```

---

## Key Types (from PRD.md)

```rust
struct Ticket {
    id: Uuid,
    source: TicketSource,
    title: String,
    description: String,
    labels: Vec<String>,
    slices: Vec<VerticalSlice>,
    status: TicketStatus,
    created_at: DateTime<Utc>,
}

enum TicketSource {
    GitHub { owner: String, repo: String, issue_number: u32 },
    Manual,
}

enum TicketStatus {
    New,
    Planning,
    InProgress,
    Review,
    Completed,
    Closed,
}
```

---

## Authentication

**Recommended: Device Flow (ticket 8.0)**
```bash
# Interactive login - opens browser, no manual token needed
nexor auth login
```

**Alternative: Environment variable**
```bash
# For CI/automation scenarios
export GITHUB_TOKEN="ghp_..."
```

The token/OAuth app needs these permissions:
- `repo` - Full repository access (read issues, create PRs)
- `read:org` - Read organization membership (for org repos)

---

## Notes

- **Rate limiting**: GitHub API has rate limits (5000 requests/hour for authenticated). Client must handle 403 responses gracefully.
- **Token security**: NEVER log or display the token. Use environment variable only.
- **Issue format**: Issues can have markdown, labels, assignees, milestones. Initial version only syncs title, body, labels.
- **PR linking**: Use "Fixes #123" in PR body to auto-link and close issues.
- **Webhooks**: Not implemented in v1. Future consideration for real-time sync.
