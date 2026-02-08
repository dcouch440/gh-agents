# Store Library

Lightweight, type-safe state management for React. Production-ready alternative to Zustand/Jotai.

## Core API

### `createStore<T>`

Creates a vanilla JS store with `getState`, `setState`, and `subscribe`.

```ts
import { createStore } from './lib'

type CounterState = { count: number }

const store = createStore<CounterState>(() => ({
  count: 0,
}))

// Read
store.getState().count

// Write
store.setState({ count: 1 })
store.setState((s) => ({ count: s.count + 1 }))

// Subscribe
const unsubscribe = store.subscribe(() => {
  console.log('State changed:', store.getState())
})
```

### `useStore<T, S>`

React hook with selector-based subscriptions (only re-render when selected slice changes).

```ts
import { useStore } from './lib'

function Counter() {
  const count = useStore(store, (s) => s.count)
  return <div>{count}</div>
}
```

**Dynamic selectors** (e.g., by ID):

```ts
const step = useStore(
  workflowStore.store,
  workflowStore.selectStepById(stepId) // ← creates new selector per stepId
)
```

### `batch`

Batch multiple updates to prevent notification storms.

```ts
import { batch } from './lib'

batch(() => {
  store.setState({ count: 1 })
  store.setState({ count: 2 })
  store.setState({ count: 3 })
})
// → Only fires one notification
```

### `shallow`

Shallow equality comparator for objects/arrays (use with `useStore`).

```ts
import { useStore, shallow } from './lib'

const state = useStore(store, (s) => ({ count: s.count, name: s.name }), shallow)
// → Only re-renders when count OR name changes (not on other field changes)
```

## Normalization

### `NormalizedMap<T>`

Immutable map with O(1) lookups and lazy array memoization (perfect for CRUD data).

```ts
import { createNormalizedMap, nmFromArray, nmSet, nmGet, toArray } from './lib'

let steps = createNormalizedMap<Step>()

// Add
steps = nmSet(steps, 'step-1', { id: 'step-1', name: 'First' })

// Get (O(1))
const step = nmGet(steps, 'step-1')

// Convert to array (memoized until next mutation)
const array = toArray(steps)
```

**Why**: Arrays re-create on every update. NormalizedMap gives stable references until the map actually changes.

### `createResourceStore`

CRUD factory for REST resources (generates selectors + async actions).

```ts
import { createResourceStore } from './lib'

const agentStore = createResourceStore({
  name: 'agents',
  api: {
    list: api.agents.list,
    get: api.agents.get,
    create: api.agents.create,
    update: api.agents.update,
    delete: api.agents.delete,
  },
})

// Selectors
const agents = useStore(agentStore.store, agentStore.selectAll)
const agent = useStore(agentStore.store, agentStore.selectById('agent-1'))

// Actions
await agentStore.fetchAll()
await agentStore.create({ name: 'New Agent' })
```

## DevTools

### `logger`

Wrap stores with beautiful console logging (color-coded, diffing, timestamps).

```ts
import { createStore, logger } from './lib'

const store = logger('myStore', createStore<State>(() => ({
  count: 0,
})))
```

**Output** (in console):

```
 myStore   increment  12:34:56 PM
  prev state { count: 0 }
  diff
    count: 0 → 1
  next state { count: 1 }
  duration: 0.12ms
```

**Configuration**:

```ts
import { configureLogger, enableLogger, disableLogger } from './lib'

// Global config
configureLogger({
  enabled: true,        // Default: import.meta.env.DEV
  collapsed: false,     // Use console.groupCollapsed
  diff: true,           // Show diff
  timestamp: true,      // Show timestamp
  colors: {
    title: '#3b82f6',
    action: '#8b5cf6',
    nextState: '#10b981',
  }
})

// Runtime toggle
enableLogger()
disableLogger()
```

**Add to all stores**:

```ts
// In each store file:
const store = logger('storeName', createStore<State>(...))
```

Logger is **zero-cost in production** (checks `import.meta.env.DEV` by default).

## Patterns

### Hand-written stores (recommended)

```ts
import { createStore, createNormalizedMap, nmSet, toArray } from './lib'

type State = {
  items: NormalizedMap<Item>
  loading: boolean
}

const store = createStore<State>(() => ({
  items: createNormalizedMap(),
  loading: false,
}))

// Selectors
const selectAll = (s: State) => toArray(s.items)
const selectById = (id: string) => (s: State) => nmGet(s.items, id)

// Actions
const fetchAll = async () => {
  store.setState({ loading: true })
  const data = await api.list()
  store.setState({ items: nmFromArray(data), loading: false })
}

export const myStore = {
  store,
  selectAll,
  selectById,
  fetchAll,
}
```

### Usage in components

```ts
import { useStore } from '@/stores/lib'
import { myStore } from '@/stores/myStore'

function MyComponent() {
  const items = useStore(myStore.store, myStore.selectAll)

  useEffect(() => {
    void myStore.fetchAll()
  }, [])

  return <div>{items.length} items</div>
}
```

## Performance Tips

1. **Use selectors** — `useStore(store, s => s.count)` only re-renders when `count` changes
2. **Use NormalizedMap** — O(1) lookups, stable array refs
3. **Use shallow equality** — `useStore(store, selector, shallow)` for object/array selectors
4. **Batch updates** — `batch(() => { ... })` for multiple setState calls
5. **Memoize derived data** — Use `useMemo` in components, not in stores

## Comparison to Zustand

| Feature | Zustand | Our Store |
|---------|---------|-----------|
| Selector-based subscriptions | ✅ | ✅ |
| TypeScript | ✅ | ✅ |
| Bundle size | 3KB | 1KB |
| Normalization helpers | ❌ | ✅ (NormalizedMap) |
| CRUD factory | ❌ | ✅ (createResourceStore) |
| Batching | Manual | ✅ Built-in |
| DevTools | ✅ (Redux DevTools) | ✅ (Console logger) |
| Middleware ecosystem | ✅ | ❌ (not needed) |

**Verdict**: Simpler, smaller, and more tailored to CRUD apps with relational data.
