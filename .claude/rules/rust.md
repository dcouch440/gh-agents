---
paths:
  - "src/**/*.rs"
---

# Rust Conventions

Applies to backend code in `src/` (Rust/Axum, edition 2021). Condensed from `research/rust-production-patterns.md`
(restorable from git history at `6b84140b~1`) and cross-checked against this codebase's actual patterns.

## Error handling

Three layers already exist — extend them, don't invent a fourth:

- `NexorError` (`src/error.rs`) — top-level, user-facing messages with recovery suggestions.
- `ServiceError` (`src/server/services/error.rs`) — domain/HTTP-agnostic. Callers include background jobs, not just handlers.
- `AppError` (`src/server/api/error/mod.rs`) — HTTP layer, implements `IntoResponse`, one variant per status code.

Rule of intent: if callers need to **match** on failure mode, add a variant to the relevant layer's `thiserror` enum.
If callers only **propagate**, use `anyhow::Result` with `.context(...)`. `ServiceError::Internal(#[from] anyhow::Error)`
is the seam where ad hoc `anyhow` errors become typed again — convert at that boundary, not deeper.

```rust
#[error(transparent)]
Internal(#[from] anyhow::Error),
```

- Error messages: lowercase, no trailing punctuation, describe only the error itself — don't embed `{source}` in the
  message string when `#[source]`/`#[from]` already chains it.
- Never `unwrap()`/`panic!` in handlers or services — propagate with `?`. Never swallow an error with `let _ = ...`;
  at minimum `tracing::warn!` it.
- Don't add a new mega error type per feature when `ServiceError`/`AppError` already covers the shape — add a variant.
- Handlers return `Result<Json<T>, AppError>` and stay thin: parse the request, call a service, return the response.
  Business logic lives in `src/server/services/`, not the handler.

## Module organization

This project intentionally uses `mod.rs` + `tests.rs` per feature folder for **every** module, not just ones with
children — see the root `CLAUDE.md` Rust Conventions. That's a deliberate deviation from the general Rust convention
of "named files over `mod.rs`" (which still applies to genuinely leaf, single-purpose files with no test file, e.g.
`src/error.rs`, `src/constants.rs`). Don't "fix" existing `mod.rs` folders to match the generic advice.

- Use `pub use` in a `mod.rs` to present a curated facade; consumers should import from the facade, not reach deep
  into submodules.
- No prelude/glob modules for application code (`use super::*;` inside `#[cfg(test)] mod tests` is the one exception).
- Fat `main.rs` is a smell — it should only wire things together; logic lives in `lib.rs`/services.

## Async & concurrency

- Always wrap external calls (DB, HTTP, LLM providers) in `tokio::time::timeout` — see `src/constants.rs` for the
  existing timeout constants before adding a new magic number.
- Prefer channels (`tokio::sync::mpsc`/`broadcast`) over shared mutexes for inter-task communication; this codebase
  already uses this actor-ish shape in several places (`EventBus`, `TaskRegistry`).
- Never hold a `MutexGuard`/`RwLockGuard` across an `.await` point — extract the value first.
- `tokio::select!`: reuse pinned futures across loop iterations (`pin!` outside the loop), don't construct new
  futures inline in the `select!` arms — state is lost each time the other branch wins. If a branch does
  non-cancel-safe work (e.g. acquiring a lock, multi-step writes), `tokio::spawn` it instead of awaiting it directly
  inside the `select!`.
- `tokio::spawn` for work that must complete regardless of the caller (e.g. a workflow execution that outlives the
  HTTP request); at minimum log errors/panics from the spawned task, don't fire-and-forget silently.
- `Rc`/`RefCell` don't work across `.await` in a multi-threaded runtime — use `Arc` and `tokio::sync::Mutex`, or move
  the state behind an actor task.

## Traits, generics & dependency injection

Trait-based DI with `#[async_trait]` + `Arc<dyn Trait>` is the established pattern for repositories
(`src/db/traits/*.rs`, implemented by `src/db/pg_repo/*.rs`) and providers (`Arc<dyn LLMProvider + Send + Sync>` in
`AppState`). Follow it for new injectable dependencies:

```rust
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AgentRepo: Send + Sync {
    async fn get_persisted_agent(&self, agent_id: Uuid) -> Result<Option<AgentRow>>;
    // ...
}
```

- Default to generics for hot-path/internal code; reach for `Arc<dyn Trait>` at service/handler boundaries where I/O
  dominates and the vtable overhead is noise — this is already how repos and providers are wired into `AppState`.
- `#[cfg_attr(test, mockall::automock)]` on the trait definition, not a hand-rolled stub, is the convention here
  (`mockall` is already a dependency). Reserve hand-written stubs for one-or-two-method traits.
- Move multi-bound generics to a `where` clause once the signature gets hard to read inline.
- Associated types when a type has exactly one natural implementation of the trait; generic parameters when a type
  could implement the trait multiple times (e.g. `From<A>` and `From<B>`).
- Don't thread the whole `AppState` into a function that only needs one repo/service out of it.

## Reducing boilerplate

- Derive `Debug` on every type. Add `Clone`/`PartialEq`/`Serialize`/`Deserialize`/`Default` as the type's usage
  actually requires them, not preemptively.
- `#[from]` on a `thiserror` variant instead of a hand-written `impl From`.
- Implement `From`, never `Into` directly (the blanket impl gives you `Into` for free). Use `TryFrom` for conversions
  that can fail — never make a `From` impl panic.
- Newtype IDs wrapping `Uuid` are the existing convention (`AgentId(pub Uuid)`, `MessageId(pub Uuid)`, etc. in
  `src/types/`) — add new domain IDs the same way rather than passing bare `Uuid` around.

## Performance

This codebase clones freely (`Arc`s, small `String`s, ids) and that's fine — don't chase micro-optimizations without
a profile. The patterns worth applying without measuring first:

- `&str` for read-only string parameters instead of `String`.
- `format!` (or a pre-sized buffer + `write!` in a genuine hot loop) instead of repeated `+`/`.push_str` concatenation.
- `bytes::Bytes` (already a dependency) for network/WebSocket byte buffers you slice — avoids copying.
- `Vec::with_capacity` when the final length is known up front.

Things like `Cow<str>`, `SmallVec`, or the typestate pattern are legitimate tools but aren't established conventions
here (no current usage) — reach for them only with a concrete reason (a profile, or a state machine with compile-time
enforceable transitions), not by default.

## Testing

- Unit tests: colocated `tests.rs` beside `mod.rs`, per the root `CLAUDE.md` — never an inline `#[cfg(test)] mod
  tests` block in `mod.rs`.
- Mock repository/provider traits with `mockall::automock` (see above) rather than hand-rolled fakes, since traits
  are already annotated for it.
- Don't test through mocks exclusively — where the code touches Postgres or another real dependency, prefer a test
  against the real thing (see existing `sqlx` test setup) over asserting against a mock's recorded calls.
- Tests must be order-independent — unique ids/fixtures per test, no shared mutable state.
