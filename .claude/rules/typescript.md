---
paths:
  - "frontend/**/*.ts"
  - "frontend/**/*.tsx"
---

# Frontend Conventions

Applies to `frontend/` (React 19 + Vite + TypeScript). Style rules are enforced by `tsc --noEmit`/`eslint .`; the
architecture sections below describe how the API layer, state system, and tests actually fit together so new code
matches the existing shape instead of introducing a second pattern.

## Style

- Strict TypeScript — no `any`, no `as` casts without a comment explaining why, no `@ts-ignore`/`@ts-expect-error`.
  (Verified: zero `any`/`@ts-ignore` in `src/`. `as` casts do occur — mostly justified, e.g. `JSON.parse(...) as T`
  or DOM event-target casts — but a few lack the explanatory comment the rule requires; don't add an uncommented one.)
- `type` over `interface`. Named exports only, one component per file.
- `null` over `undefined` for domain/app state (`Agent.output_schema_id: string | null`, store `error: string |
  null`). Exception: anything backed by `Map`/`NormalizedMap` semantics returns `T | undefined` for "not found"
  (`selectById`, `nmGet`) — match `Map.get`'s convention there rather than coercing to `null`.
- Components: `function ComponentName(...)` declarations, exported at the bottom of the file (`export {
  ComponentName }`), not `export function`. Everything else (hooks, store actions, helpers) is `const x = (...) =>
  ...`.
- No `React.FC`, no `forwardRef`, no external state libraries (Redux/Zustand/Jotai/Context-as-store) — use this
  project's own store library (`src/stores/lib/`, see below).
- Components are stateless and pure; pages own data/state and pass it down.
- `Collections` (`@/utils/collections`) for array operations — `mapBy`, `filterMap`, `keyBy`, `groupBy`, `partition`,
  `resolveKeys`, `sumBy`, `dedup`, etc. Never a raw `.filter().map()` chain or `.find()` inside a loop; use
  `Collections.filterMap` / `Collections.resolveKeys` instead.
- ESLint is strict (React 19 rules) — no `eslint-disable`, fix the underlying code.

## API layer (`src/api/`)

`client.ts` is the low-level fetch wrapper (`api.get/post/patch/put/del`) — request dedup for in-flight GETs, retry
with backoff (skips 4xx), `AbortController` timeout, auth token injected from `localStorage` in `buildHeaders`.
`api.ts` builds typed, namespaced, `Object.freeze`d endpoint groups on top of it, keyed by domain (`agents`, `tools`,
`documents`, `sessions`, `workflows`, ...), and re-exports the low-level methods alongside them on one `api` object.
Full docs: `src/api/README.md` and `src/api/EXAMPLES.md` — read those before extending this layer.

Adding an endpoint:

```ts
// src/api/api.ts — inside the relevant namespace
get: (id: string, config?: RequestConfig) => baseApi.get<Agent>(API.AGENT(id), config),
create: (body: CreateAgentRequest, config?: RequestConfig) => baseApi.post<Agent>(API.AGENTS, body, config),
```

- URL paths always come from the central `API` constant map (`@/constants`) — never an inline string literal.
- Request/response types live in `src/types/<domain>.ts` as `type`, one file per domain; request types are usually
  `Partial<CreateXRequest>` for updates. Define a type inline in `api.ts` only when it's genuinely call-site-specific
  (`LoginResponse`, `MeResponse`), not a reusable domain model.
- Use `baseApi.get/post/patch/put/del` for genuinely one-off/uncatalogued endpoints — this is a documented escape
  hatch (see README, "Low-Level API"), not a violation of "use typed endpoints." Prefer adding a namespaced method
  when the endpoint will be called from more than one place.
- Errors are a discriminated `ApiError` (`network_error | timeout_error | abort_error | http_error |
  rate_limit_error | parse_error`) with narrowing guards in `src/api/guards.ts` (`isHttpError`, `hasStatus(e, 404)`,
  `isClientError`, ...) — use those instead of inspecting `error.message`/`status` by hand.
- Global 401 handling is centralized in `src/api/authInterceptor.ts` (calls `authStore.logout()`), not per-call —
  don't add ad hoc 401 handling in a store or component.
- Streaming endpoints use `createSSEStream` (`src/api/sse.ts`), which returns an abort function.

## State system (`src/stores/`)

Custom store library at `src/stores/lib/` (`createStore.ts`, `useStore.ts`, `NormalizedMap.ts`,
`createResourceStore.ts`) — read `src/stores/lib/README.md` first; it's written as a Zustand/Jotai replacement and
explains the primitives below.

```ts
const store = logger('agentStore', createStore<AgentState>(() => ({
  items: createNormalizedMap<Agent>(), stats: null, loading: false, error: null,
})))

const selectAll = (s: AgentState): Agent[] => toArray(s.items)
const selectById = (id: string) => (s: AgentState): Agent | undefined => nmGet(s.items, id)

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.agents.list()
    store.setState({ items: nmFromArray(data.agents), stats: data.stats, loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError('agents', e) })
  }
}

export const agentStore = { store, selectAll, selectById, fetchAll, /* ... */ }
```

- Every resource store carries `loading: boolean` and `error: string | null`, set at the start/end of each async
  action; build `error` with the shared `extractError` helper (`stores/lib/extractError.ts`), don't stringify the
  caught error by hand.
- Entity collections use `NormalizedMap<T>` (`nmSet`/`nmGet`/`nmDelete`/`nmFromArray`/`toArray`), not a raw array —
  it gives O(1) lookup and stable references.
- Mutating actions that hit the API (`create`/`update`/`remove`) apply the change optimistically, then roll back to
  the previous state on failure and rethrow — see `agentStore.remove` for the pattern (snapshot `prev`, `setState`
  optimistically, `catch` restores `prev` and re-throws).
- `createResourceStore` (`stores/lib/createResourceStore.ts`) generates plain CRUD (`selectAll/selectById/fetchAll/
  create/update/remove`) — use it for a plain CRUD resource. Most stores in this repo are hand-written instead
  because they carry resource-specific state beyond CRUD (e.g. `toolsByAgent`); hand-writing in the same shape as
  `agentStore.ts` is the norm, not a deviation.
- Wrap `createStore(...)` in `logger('storeName', ...)` (`stores/lib/devtools.ts`) for dev-mode console logging —
  every existing store does this.
- Derive computed values in selectors or `useMemo` in the component — never inside the store itself
  ("memoize derived data in components, not stores," per the store-lib README).
- A store talks to the API layer directly from its action functions (`await api.<namespace>.<method>()`); components
  never call `api.*` directly — they call a store action.
- Export one object per store file (`export { agentStore }`) plus its state type (`export type { AgentState }`);
  register it in `src/stores/index.ts`. A store large enough to need internal submodules gets its own folder
  (`stores/workflowStore/`) with `selectors.ts`/`hydrate.ts` inside, same external shape.

## Tests

Vitest, `jsdom`, globals on (`describe`/`it`/`vi` used without importing, though explicit imports also appear —
either is fine). Colocate `Foo.test.tsx` next to `Foo.tsx`.

Mock the API layer, not the network — `vi.mock('@/api', ...)` with `vi.hoisted` for the mock functions (required
since `vi.mock` factories are hoisted above the mocked-in variables):

```ts
const { mockList, mockCreate } = vi.hoisted(() => ({ mockList: vi.fn(), mockCreate: vi.fn() }))
vi.mock('@/api', () => ({ api: { agents: { list: mockList, create: mockCreate } } }))
```

- Store tests: reset state in `beforeEach` via `store.setState({ ...initialShape })`, call the action, assert on
  `store.getState()`. Assert rollback-on-failure explicitly for optimistic actions.
- Component tests: `@testing-library/react` + `@testing-library/user-event`. Use `render` from `src/test/render.tsx`
  (wraps MUI's `ThemeProvider`) for anything rendering themed components; a bare `render` from
  `@testing-library/react` is fine for logic that doesn't touch MUI.
- Page-level tests mock the store module directly (`vi.mock('@/stores/xStore', ...)`), stubbing
  `selectAll`/`selectLoading`/`selectError`/actions as functions closing over reassignable module-level `let`s so
  `beforeEach` can reset them between tests.
- `src/test/fixtures.ts` holds typed constant fixtures (`export const mockAgent: Agent = {...}`), including variants
  built via spread (`{ ...mockAgent, id: '...' }`). Use it for anything reusable across test files; an inline fixture
  local to one test file is fine when nothing else needs it.
- `src/test/setup.ts` stubs `ResizeObserver`/`matchMedia` for jsdom — don't re-stub these per test file.
