# Milestone 17: SQLite to PostgreSQL Migration

> Pure database swap — same schema, no new features. Hard cut, no dual-driver support.

## Goal

Replace SQLite with PostgreSQL across the entire codebase. Native Postgres types (UUID, TIMESTAMPTZ, BOOLEAN, JSONB), `sqlx::migrate!` macro, Docker Compose for local dev, per-test database isolation.

**Checkpoint**: `docker compose up` starts app connected to Postgres, all tests pass against Postgres.

---

## Scope

- 8 tickets, ~32 slices
- 14 migration files rewritten as Postgres DDL
- 4 db module files updated
- 10 consumer files updated
- Cargo.toml sqlx features changed
- Docker Compose updated with Postgres service
- All test infrastructure migrated

## Type Mapping

| SQLite | PostgreSQL |
|--------|-----------|
| `TEXT` UUIDs | Native `UUID` type |
| `TEXT` RFC3339 timestamps | `TIMESTAMPTZ` |
| `INTEGER` 0/1 booleans | Native `BOOLEAN` |
| `TEXT` JSON strings | `JSONB` |
| `datetime('now')` | `NOW()` |
| `?` param markers | `$1, $2, ...` |
| Custom migration runner (split on `;`) | `sqlx::migrate!` macro |
| `SqlitePool` (5 connections) | `PgPool` (10 connections) |
| `.nexor/state.db` file | `DATABASE_URL` env var |
| `tempfile::TempDir` in tests | Per-test Postgres databases |

## Dependency Graph

```
17.1 (Infrastructure)
  └→ 17.2 (Migration Files)
      └→ 17.3 (Pool/Init)
          ├→ 17.4 (Queries: tasks/chat/auth)  ← parallel
          └→ 17.5 (Queries: refactor/prd)     ← parallel
              └→ 17.6 (Consumer Updates)
                  └→ 17.7 (Tests)
                      └→ 17.8 (Cleanup & Docs)
```

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|-------------|
| 17.1 | Infrastructure & Dependencies | 4 | None |
| 17.2 | Migration Files Rewrite | 4 | 17.1 |
| 17.3 | Connection Pool & Init | 3 | 17.2 |
| 17.4 | Query Rewrites: Tasks/Chat/Auth | 5 | 17.3 |
| 17.5 | Query Rewrites: Refactor/PRD | 4 | 17.3 |
| 17.6 | Consumer File Updates | 4 | 17.4, 17.5 |
| 17.7 | Test Infrastructure | 4 | 17.6 |
| 17.8 | Cleanup & Documentation | 4 | 17.7 |

## Key Design Decisions

1. **Hard cut** — No feature flags, no dual-driver. SQLite removed entirely.
2. **sqlx::migrate!** — Replace custom migration runner with sqlx's built-in system.
3. **Native types** — Row structs use `uuid::Uuid`, `DateTime<Utc>`, `bool`, `serde_json::Value` directly.
4. **Docker Compose** — `postgres:16-alpine` with health check for local dev.
5. **Test isolation** — Each test creates its own database via `CREATE DATABASE`, drops on teardown.
6. **Data migration tool** — Optional `migrate_sqlite_to_pg` binary for existing users.
