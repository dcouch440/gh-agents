# Security Hardening — Remediation Plan

**Priority:** Critical
**Identified:** 2026-02-06
**Source:** Comprehensive 4-domain security audit (injection, auth, secrets, resource exhaustion)

---

## Phase 1 — Immediate (CRITICAL)

> These can all run in parallel. No dependencies between them.

### 1A. Revoke Committed API Key

**Severity:** CRITICAL
**Files:** `.env`, git history
**Effort:** 30 min

A real Anthropic API key (`sk-ant-api03-...`) is committed in `.env` and exists in git history.

**Tasks:**
- [ ] Revoke the key immediately via Anthropic dashboard
- [ ] Generate a new key, inject via environment (not `.env`)
- [ ] Scrub `.env` from git history with `git filter-repo` or BFG Repo-Cleaner
- [ ] Verify `.gitignore` catches all `.env` variants (root already does; add to `frontend/.gitignore`)

---

### 1B. Fix Agent IDOR — Missing Ownership Checks

**Severity:** CRITICAL
**File:** `src/server/api/agents/mod.rs` (lines 207-304)
**Effort:** 1-2 hours

`get_agent`, `update_agent`, `delete_agent` accept any UUID without verifying the authenticated user owns the agent. Any logged-in user can read/modify/delete any other user's agents.

**Tasks:**
- [ ] Add ownership check to `get_agent()` — load agent, compare `user_id` field against `auth.user_id.0`, return 404 if mismatch
- [ ] Add ownership check to `update_agent()` — same pattern
- [ ] Add ownership check to `delete_agent()` — same pattern
- [ ] Change `_auth` to `auth` where ownership is now used
- [ ] Write tests: authenticated user can access own agent, gets 404 for another user's agent

**Pattern:**
```rust
let row = state.repo().get_persisted_agent(id).await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;
if row.user_id != auth.user_id.0 {
    return Err(StatusCode::NOT_FOUND);
}
```

> Note: Return 404 (not 403) to avoid confirming resource existence.

---

### 1C. Fix Tool IDOR — Missing Ownership or Admin Restriction

**Severity:** CRITICAL
**File:** `src/server/api/tools/mod.rs` (lines 105-232, 245-305)
**Effort:** 2-3 hours

Tools are currently global — any authenticated user can CRUD any tool, affecting all users' agents. The `get_agent_tools` and `set_agent_tools` endpoints also skip agent ownership checks.

**Decision needed:** Are tools user-scoped or admin-only?

**Option A — User-scoped tools:**
- [ ] Add `user_id` column to `tools` table (migration)
- [ ] Filter all tool queries by `user_id`
- [ ] Add ownership checks to get/update/delete handlers
- [ ] Seed built-in tools per user on registration (already done for some)

**Option B — Admin-only tool management (simpler):**
- [ ] Add `is_admin` field to users or create admin role
- [ ] Restrict create/update/delete to admin users
- [ ] Keep read (list/get) available to all authenticated users
- [ ] Add admin check middleware or guard

**Either way:**
- [ ] Add ownership check to `get_agent_tools()` — verify agent belongs to user
- [ ] Add ownership check to `set_agent_tools()` — verify agent belongs to user

---

### 1D. Fix Agent Mode IDOR — Missing Agent Ownership Check

**Severity:** CRITICAL
**File:** `src/server/api/sessions/mod.rs` (lines 118-195)
**Effort:** 1-2 hours

`list_agent_modes`, `create_agent_mode`, `delete_agent_mode` accept any agent UUID without verifying ownership.

**Tasks:**
- [ ] In `list_agent_modes()` — load agent, verify ownership before listing modes
- [ ] In `create_agent_mode()` — load agent, verify ownership before creating mode
- [ ] In `delete_agent_mode()` — load mode, resolve agent, verify ownership before deleting
- [ ] Change `_auth` to `auth` in all three handlers
- [ ] Write tests for ownership enforcement

**Helper pattern (reusable across 1B/1C/1D):**
```rust
/// Verify the authenticated user owns this agent. Returns the agent row or 404.
async fn verify_agent_ownership(
    repo: &dyn ServerRepo,
    agent_id: Uuid,
    user_id: Uuid,
) -> Result<AgentRow, StatusCode> {
    let agent = repo.get_persisted_agent(agent_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if agent.user_id != user_id {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(agent)
}
```

---

## Phase 2 — This Sprint (HIGH)

> These can all run in parallel. No dependencies on Phase 1.

### 2A. Fix Error Leakage — Stop Sending Raw DB Errors to Clients

**Severity:** HIGH
**File:** `src/server/api/auth/mod.rs` (lines 102, 120, 126, 169, 176, 181, 187)
**Effort:** 1-2 hours

Seven `.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))` calls send raw database errors (schema names, query structure) directly to HTTP clients.

**Tasks:**
- [ ] Replace all `e.to_string()` error responses with generic message
- [ ] Log the actual error server-side via `tracing::error!`
- [ ] Audit all other API modules for the same pattern (`grep -rn "e.to_string()" src/server/api/`)
- [ ] Fix any other occurrences found

**Pattern:**
```rust
.map_err(|e| {
    tracing::error!(error = %e, "database operation failed");
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".into())
})?
```

---

### 2B. Enforce CORS in Production

**Severity:** HIGH
**File:** `src/server/mod.rs` (lines 449-488)
**Effort:** 30 min

When `CORS_ORIGINS` is unset, the server defaults to `allow_origin(Any)` with only a warning. A forgotten env var in production enables CSRF.

**Tasks:**
- [ ] In `build_cors_layer()`, check `RUST_ENV` — if production, panic when `CORS_ORIGINS` is unset
- [ ] Keep permissive default only for development
- [ ] Add test verifying production panics without CORS_ORIGINS

**Pattern:**
```rust
_ => {
    let is_production = std::env::var(ENV_RUST_ENV)
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false);
    if is_production {
        panic!("CORS_ORIGINS must be set in production");
    }
    warn!("CORS_ORIGINS not set — allowing all origins (dev mode)");
    CorsLayer::permissive()
}
```

---

### 2C. Explicit JWT Algorithm

**Severity:** HIGH
**File:** `src/server/auth/mod.rs` (lines 59-73)
**Effort:** 30 min

JWT encoding uses `Header::default()` and decoding uses `Validation::default()` without explicitly locking to HS256. The `jsonwebtoken` crate defaults are safe today, but explicit is defense-in-depth.

**Tasks:**
- [ ] Set `Header { alg: Algorithm::HS256, ..Default::default() }` in `encode()`
- [ ] Set `Validation::new(Algorithm::HS256)` in `decode()`
- [ ] Add JWT algorithm mismatch test

---

## Phase 3 — Next Sprint (MEDIUM)

> These can all run in parallel.

### 3A. Strengthen Password Policy

**Severity:** MEDIUM
**File:** `src/server/api/auth/mod.rs` (lines 111, 153)
**Effort:** 1 hour

Current minimum is 8 characters with no complexity requirements.

**Tasks:**
- [ ] Increase minimum to 12 characters
- [ ] Add at least one complexity rule (e.g., must contain uppercase + lowercase + digit)
- [ ] Extract validation to a shared `validate_password()` function
- [ ] Update error messages to describe requirements
- [ ] Add tests for boundary cases

---

### 3B. Cap Response Stream Buffer

**Severity:** MEDIUM
**File:** `src/server/state/mod.rs` (~line 547)
**Effort:** 1 hour

`BufferedStream.buffer: Vec<StreamChunk>` grows without bound. A very long LLM response could accumulate excessive memory.

**Tasks:**
- [ ] Add `MAX_STREAM_BUFFER_CHUNKS` constant (e.g., 10,000)
- [ ] In `send_stream_chunk()`, drop oldest chunks when buffer exceeds max (ring buffer pattern) or stop buffering after limit
- [ ] Add cleanup timeout — auto-remove streams older than N minutes

---

### 3C. Production-Require DATABASE_URL

**Severity:** MEDIUM
**File:** `src/types/config.rs` (line 193)
**Effort:** 30 min

Default DB URL `postgres://nexor:nexor@localhost:5432/nexor` is hardcoded for dev convenience. Production should require the env var.

**Tasks:**
- [ ] Check `RUST_ENV` — if production, panic when `DATABASE_URL` is unset
- [ ] Add explicit `// DEV ONLY` comment on the default constant
- [ ] Add test for production enforcement

---

## Phase Summary

| Phase | Items | Parallel? | Effort | Timeline |
|-------|-------|-----------|--------|----------|
| **1 (Critical)** | 1A, 1B, 1C, 1D | All parallel | ~6 hours total | Today |
| **2 (High)** | 2A, 2B, 2C | All parallel | ~3 hours total | This sprint |
| **3 (Medium)** | 3A, 3B, 3C | All parallel | ~3 hours total | Next sprint |

## Out of Scope (Acceptable Risk)

These were flagged at LOW severity and don't warrant remediation work:

- **WS token in query param** — Intentional for SSE compatibility, acceptable with HTTPS
- **Email enumeration on registration** — Standard behavior, not a meaningful attack vector here
- **Test fixture API key strings** — Fake keys in test code, no real exposure
