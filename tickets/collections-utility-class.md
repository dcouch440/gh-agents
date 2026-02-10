# Collections Utility Class — Static Array Algorithms & Frontend Migration

## Context

The frontend codebase uses `.map()`, `.filter()`, `.reduce()`, `.find()`, and `.sort()` extensively. While individually correct, many call sites iterate the same array multiple times, chain operations that could be single-pass, or use `Array.find()`/`Array.includes()` inside loops (producing O(n*m) or O(n^2) patterns). These are small inefficiencies today but will compound as workflow sizes, table row counts, and command lists grow.

This ticket creates a `Collections` static utility class with battle-tested algorithms that replace scattered iteration patterns with single-pass, memory-efficient alternatives. The class then gets adopted across the codebase wherever the patterns exist.

**Key decisions:**
- Static class with pure functions (no instantiation, no state)
- Generic methods that work with any array type
- All methods return new arrays/maps (immutable — no mutation)
- Each method independently unit tested
- Migration is incremental — adopt per-file, not a big-bang refactor

---

## Part 1: Implement `Collections` Utility Class

> **Effort:** 2-3 hours
> **Dependencies:** None

### File

`frontend/src/utils/collections.ts`

### Methods

#### `Collections.indexBy<T, K>(items: T[], keyFn: (item: T) => K): Map<K, T>`

Builds a `Map<K, T>` from an array in a single `for` loop. Last item wins on key collision.

**Replaces:** `new Map(items.map(i => [i.id, i]))` — eliminates the intermediate tuple array allocation.

```typescript
static indexBy<T, K>(items: readonly T[], keyFn: (item: T) => K): Map<K, T> {
  const map = new Map<K, T>();
  for (let i = 0; i < items.length; i++) {
    map.set(keyFn(items[i]), items[i]);
  }
  return map;
}
```

---

#### `Collections.groupBy<T, K>(items: T[], keyFn: (item: T) => K): Map<K, T[]>`

Groups items into `Map<K, T[]>` in a single pass. Each key maps to all items that produced it.

**Replaces:** Manual `forEach` + conditional map insertion patterns.

---

#### `Collections.filterMap<T, U>(items: T[], fn: (item: T, index: number) => U | null): U[]`

Combined filter + map in a single pass. The callback returns `null` to skip or a transformed value to include.

**Replaces:** `.filter(predicate).map(transform)` chains that iterate twice.

```typescript
static filterMap<T, U>(items: readonly T[], fn: (item: T, index: number) => U | null): U[] {
  const result: U[] = [];
  for (let i = 0; i < items.length; i++) {
    const mapped = fn(items[i], i);
    if (mapped !== null) {
      result.push(mapped);
    }
  }
  return result;
}
```

---

#### `Collections.partition<T>(items: T[], predicate: (item: T) => boolean): [T[], T[]]`

Splits an array into `[matches, rest]` in a single pass.

**Replaces:** Two separate `.filter()` calls with inverted predicates.

---

#### `Collections.sumBy<T>(items: T[], valueFn: (item: T) => number): number`

Sums a numeric field in a single `for` loop.

**Replaces:** `.reduce((sum, item) => sum + item.field, 0)`.

---

#### `Collections.multiAggregate<T>(items: T[], ...fns: Array<(item: T) => number>): number[]`

Computes multiple numeric aggregations in a single pass. Returns an array of results matching the order of input functions.

**Replaces:** Multiple `.reduce()` calls over the same array (e.g., three separate passes for input tokens, output tokens, call count).

```typescript
static multiAggregate<T>(items: readonly T[], ...fns: Array<(item: T) => number>): number[] {
  const results = new Array<number>(fns.length).fill(0);
  for (let i = 0; i < items.length; i++) {
    for (let j = 0; j < fns.length; j++) {
      results[j] += fns[j](items[i]);
    }
  }
  return results;
}
```

---

#### `Collections.toSet<T, K>(items: T[], keyFn?: (item: T) => K): Set<K>`

Creates a `Set` from an array in a single pass, optionally extracting a key.

**Replaces:** `new Set(items.map(i => i.id))` — eliminates intermediate array.

---

#### `Collections.uniqueBy<T, K>(items: T[], keyFn: (item: T) => K): T[]`

Deduplicates by key using a Set internally. First item wins on collision.

**Replaces:** `.filter()` with `indexOf` or manual Set tracking.

---

#### `Collections.sortBy<T>(items: T[], ...comparators: Array<(a: T, b: T) => number>): T[]`

Immutable sort with chained comparators. Creates a copy, then sorts with a composite comparator that falls through to the next comparator on ties.

**Replaces:** `[...arr].sort(cmp)` scattered across the codebase. Provides multi-key sorting without manual comparator nesting.

```typescript
static sortBy<T>(items: readonly T[], ...comparators: Array<(a: T, b: T) => number>): T[] {
  const copy = items.slice();
  copy.sort((a, b) => {
    for (let i = 0; i < comparators.length; i++) {
      const result = comparators[i](a, b);
      if (result !== 0) return result;
    }
    return 0;
  });
  return copy;
}
```

---

#### `Collections.keyBy<T>(items: T[], keyFn: (item: T) => string): Record<string, T>`

Like `indexBy` but returns a plain object instead of a Map. Useful for serializable state.

---

#### `Collections.buildLookup<T, V>(items: T[], keyFn: (item: T) => string, valueFn: (item: T) => V): Map<string, V>`

Builds a lookup map where both key and value are derived. Single pass.

**Replaces:** `new Map(items.map(i => [i.id, { name: i.name, model_id: i.model_id }]))` — avoids intermediate tuple array.

---

### Tests

`frontend/src/utils/collections.test.ts`

Each method needs tests for:
- Empty array input
- Single item
- Multiple items (happy path)
- Key collision behavior (indexBy: last wins, uniqueBy: first wins)
- Type safety (generics work correctly)
- Immutability (input array not mutated)

---

## Part 2: Priority 1 Migrations — Fix Actual Performance Issues

> **Effort:** 1-2 hours
> **Dependencies:** Part 1

These are places where the current code has measurable inefficiency.

### 2A. `useCommandPalette.ts:47` — O(n*m) find-in-loop

**Current:**
```typescript
const recent = recentIds
  .map((id) => commands.find((c) => c.id === id))
  .filter((c): c is CommandItem => c !== undefined);
```

For each `recentId`, scans the entire `commands` array. With 10 recent IDs and 50 commands, that's 500 comparisons instead of 60.

**Migration:**
```typescript
const commandsById = Collections.indexBy(commands, (c) => c.id);
const recent = Collections.filterMap(recentIds, (id) => {
  const cmd = commandsById.get(id);
  return cmd ? { ...cmd, group: 'recent' as const } : null;
});
```

---

### 2B. `Table.tsx:142` — O(n^2) includes-in-loop

**Current:**
```typescript
const isDifferent =
  currentSelection.length !== selectedRows.length ||
  currentSelection.some((key) => !selectedRows.includes(key));
```

`Array.includes()` is O(n), called inside `.some()` which is O(n). Total: O(n^2).

**Migration:**
```typescript
const selectedSet = Collections.toSet(selectedRows, (k) => k);
const isDifferent =
  currentSelection.length !== selectedSet.size ||
  currentSelection.some((key) => !selectedSet.has(key));
```

---

### 2C. `TaskQueueStatus.tsx:63-65` — Unmemoized filter+sort in render

**Current:**
```typescript
const active = tasks
  .filter((t) => t.status !== 'completed')
  .sort((a, b) => PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority]);
```

This runs on every render, creating a new filtered array and sorting it even if `tasks` hasn't changed.

**Migration:** Wrap in `useMemo` and use `Collections.sortBy`:
```typescript
const active = useMemo(
  () => Collections.sortBy(
    tasks.filter((t) => t.status !== 'completed'),
    (a, b) => PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority],
  ),
  [tasks],
);
```

---

## Part 3: Priority 2 Migrations — Consolidate Repeated Patterns

> **Effort:** 2-3 hours
> **Dependencies:** Part 1

### 3A. `TokenUsageStatus.tsx:15-17` — Triple reduce

**Current:**
```typescript
const totalInput = usage.reduce((s, r) => s + r.total_input, 0);
const totalOutput = usage.reduce((s, r) => s + r.total_output, 0);
const totalCalls = usage.reduce((s, r) => s + r.call_count, 0);
```

Three passes over the same array.

**Migration:**
```typescript
const [totalInput, totalOutput, totalCalls] = Collections.multiAggregate(
  usage,
  (r) => r.total_input,
  (r) => r.total_output,
  (r) => r.call_count,
);
```

---

### 3B. `WorkflowCanvas.tsx:76-121` — Five lookup map constructions

**Current:** Five separate `useMemo` blocks each with `new Map(items.map(...))`.

**Migration:** Replace inner expressions with `Collections.indexBy` or `Collections.buildLookup`:
```typescript
const agentLookup = useMemo(
  () => Collections.buildLookup(agents, (a) => a.id, (a) => ({ name: a.name, model_id: a.model_id })),
  [agents],
);
```

---

### 3C. `mappers.ts:43-52` — Manual upstream grouping

**Current:**
```typescript
const upstreamMap = new Map<string, string[]>();
for (const edge of lookups.edges) {
  const list = upstreamMap.get(edge.to_step_id) ?? [];
  list.push(edge.from_step_id);
  upstreamMap.set(edge.to_step_id, list);
}
```

**Migration:**
```typescript
const upstreamMap = Collections.groupBy(lookups.edges, (e) => e.to_step_id);
// Then map values to extract from_step_id where needed
```

---

### 3D. `CommandPaletteContext.tsx:54-59` — Map + filter dedup

**Current:**
```typescript
const ids = new Set(newCommands.map((c) => c.id));
const filtered = prev.filter((c) => !ids.has(c.id));
return [...filtered, ...newCommands];
```

**Migration:**
```typescript
const idSet = Collections.toSet(newCommands, (c) => c.id);
const filtered = prev.filter((c) => !idSet.has(c.id));
return [...filtered, ...newCommands];
```

---

### 3E. `ReviewQueuePage.tsx:38-43` — Filter + map chain

**Current:**
```typescript
chatMessages = chat.messages
  .filter((m) => m.role === 'user' || m.role === 'assistant')
  .map((m) => ({...}));
```

**Migration:**
```typescript
chatMessages = Collections.filterMap(chat.messages, (m) =>
  m.role === 'user' || m.role === 'assistant'
    ? { role: m.role, content: m.content }
    : null,
);
```

---

## Files Involved

| File | Change Type |
|------|-------------|
| `utils/collections.ts` | **NEW** — Static utility class |
| `utils/collections.test.ts` | **NEW** — Unit tests |
| `hooks/useCommandPalette.ts` | Migrate find-in-loop to Map lookup |
| `components/primitives/Table/Table.tsx` | Migrate includes-in-loop to Set |
| `components/dashboard/TaskQueueStatus.tsx` | Add useMemo + Collections.sortBy |
| `components/dashboard/TokenUsageStatus.tsx` | Migrate triple reduce to multiAggregate |
| `components/canvas/WorkflowCanvas.tsx` | Migrate lookup map construction |
| `components/canvas/mappers.ts` | Migrate upstream grouping |
| `contexts/CommandPaletteContext.tsx` | Migrate dedup pattern |
| `pages/ReviewQueuePage.tsx` | Migrate filter+map chain |

## Acceptance Criteria

- [ ] `Collections` class exists at `frontend/src/utils/collections.ts` with all 11 static methods
- [ ] Full unit test suite in `frontend/src/utils/collections.test.ts`
- [ ] All Priority 1 migrations complete (useCommandPalette, Table, TaskQueueStatus)
- [ ] All Priority 2 migrations complete
- [ ] No `Array.find()` or `Array.includes()` called inside loops anywhere in the codebase
- [ ] `npx tsc --noEmit` passes with zero errors
- [ ] `npx eslint .` passes with zero warnings
- [ ] All existing tests pass (`npx vitest run`)
- [ ] No runtime regressions in canvas, table, command palette, or dashboard
