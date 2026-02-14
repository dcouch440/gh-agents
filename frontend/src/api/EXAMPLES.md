# API Client Usage Examples

Real-world examples of using the API client.

## Basic CRUD Operations

### Listing Resources

```typescript
import { api } from '@/api'

// List all agents
const { agents } = await api.agents.list()

// List with custom config
const { agents } = await api.agents.list({
  timeout: 5000,
  retries: 2,
})
```

### Getting a Single Resource

```typescript
// Get a specific agent
const agent = await api.agents.get('agent-123')

// With custom timeout
const agent = await api.agents.get('agent-123', {
  timeout: 10000,
})
```

### Creating Resources

```typescript
// Create an agent
const newAgent = await api.agents.create({
  name: 'My Agent',
  model: 'claude-3-opus',
  tier: 'research',
  system_prompt: 'You are a helpful assistant',
})

// Create a task
const task = await api.tasks.create({
  title: 'Implement feature X',
  description: 'Add authentication to the API',
  priority: 'high',
  tier: 'dev',
})
```

### Updating Resources

```typescript
// Update an agent
const updated = await api.agents.update('agent-123', {
  name: 'Updated Name',
  system_prompt: 'New prompt',
})

// Update a session
await api.sessions.update('session-456', {
  title: 'Updated Session Title',
})
```

### Deleting Resources

```typescript
// Delete an agent
await api.agents.delete('agent-123')

// Delete a document
await api.documents.delete('doc-789')
```

## Error Handling Patterns

### Basic Error Handling

```typescript
import { api, ApiError } from '@/api'

try {
  const agent = await api.agents.get('invalid-id')
} catch (error) {
  if (error instanceof ApiError) {
    console.error(`API Error: ${error.message}`)
    console.error(`Status: ${error.status}`)
    console.error(`Type: ${error.type}`)
  } else {
    console.error('Unexpected error:', error)
  }
}
```

### Using Type Guards

```typescript
import { api } from '@/api'
import { isHttpError, hasStatus, isNetworkError, isClientError, isServerError } from '@/api'

try {
  const agent = await api.agents.get('agent-123')
} catch (error) {
  if (hasStatus(error, 404)) {
    alert('Agent not found')
  } else if (hasStatus(error, 401)) {
    window.location.href = '/login'
  } else if (hasStatus(error, 403)) {
    alert('You do not have permission to view this agent')
  } else if (isClientError(error)) {
    alert('Bad request')
  } else if (isServerError(error)) {
    alert('Server error. Please try again later.')
  } else if (isNetworkError(error)) {
    alert('Network connection failed. Please check your internet.')
  }
}
```

### Handling Specific Error Types

```typescript
import { api, ApiError } from '@/api'

try {
  const agent = await api.agents.get('agent-123')
} catch (error) {
  if (error instanceof ApiError) {
    switch (error.type) {
      case 'http_error':
        console.log(`HTTP ${error.status}: ${error.statusText}`)
        break
      case 'network_error':
        alert('Network connection failed. Please check your internet.')
        break
      case 'timeout_error':
        alert('Request timed out. Please try again.')
        break
      case 'abort_error':
        console.log('Request was cancelled')
        break
    }
  }
}
```

### Retry on Failure

```typescript
// Automatically retry up to 3 times
const agent = await api.agents.get('agent-123', {
  retries: 3,
  retryDelay: 1000, // Start with 1s, then 2s, then 4s (exponential backoff)
})
```

## React Hooks Integration

### Basic Data Fetching Hook

```typescript
import { useState, useEffect } from 'react'
import { api } from '@/api'
import { isAbortError } from '@/api'
import type { Agent } from '@/types'

function useAgents() {
  const [agents, setAgents] = useState<Agent[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const controller = new AbortController()

    const load = async () => {
      setLoading(true)
      setError(null)
      try {
        const { agents } = await api.agents.list({
          signal: controller.signal,
        })
        setAgents(agents)
      } catch (err) {
        if (!isAbortError(err)) {
          setError(err instanceof Error ? err.message : 'Unknown error')
        }
      } finally {
        setLoading(false)
      }
    }

    void load()

    return () => {
      controller.abort()
    }
  }, [])

  return { agents, loading, error }
}
```

### Mutation Hook

```typescript
import { useState, useCallback } from 'react'
import { api, ApiError } from '@/api'
import type { Agent, CreateAgentRequest } from '@/types'

function useCreateAgent() {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const create = useCallback(async (body: CreateAgentRequest): Promise<Agent | null> => {
    setLoading(true)
    setError(null)
    try {
      const agent = await api.agents.create(body)
      return agent
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : 'Failed to create agent'
      setError(msg)
      return null
    } finally {
      setLoading(false)
    }
  }, [])

  return { create, loading, error }
}
```

### Using the Hooks in a Component

```typescript
function AgentsList() {
  const { agents, loading, error } = useAgents()
  const { create, loading: creating } = useCreateAgent()

  if (loading) return <div>Loading...</div>
  if (error) return <div>Error: {error}</div>

  const handleCreate = async () => {
    const agent = await create({
      name: 'New Agent',
      model: 'claude-3-opus',
      tier: 'research',
      system_prompt: 'You are helpful',
    })

    if (agent) {
      alert(`Created agent: ${agent.name}`)
    }
  }

  return (
    <div>
      <button onClick={handleCreate} disabled={creating}>
        Create Agent
      </button>
      <ul>
        {agents.map((agent) => (
          <li key={agent.id}>{agent.name}</li>
        ))}
      </ul>
    </div>
  )
}
```

## Request Cancellation

### Cancel on Component Unmount

```typescript
import { isAbortError } from '@/api'

useEffect(() => {
  const controller = new AbortController()

  const fetchData = async () => {
    try {
      const { agents } = await api.agents.list({
        signal: controller.signal,
      })
      setAgents(agents)
    } catch (error) {
      if (isAbortError(error)) {
        console.log('Request cancelled')
      } else {
        console.error('Error:', error)
      }
    }
  }

  void fetchData()

  return () => {
    controller.abort()
  }
}, [])
```

### Cancel on User Action

```typescript
function SearchAgents() {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<Agent[]>([])
  const abortControllerRef = useRef<AbortController | null>(null)

  const search = async (searchQuery: string) => {
    // Cancel previous request
    abortControllerRef.current?.abort()

    // Create new controller for this request
    const controller = new AbortController()
    abortControllerRef.current = controller

    try {
      const { agents } = await api.agents.list({
        signal: controller.signal,
      })
      const filtered = agents.filter((a) => a.name.includes(searchQuery))
      setResults(filtered)
    } catch (error) {
      if (!isAbortError(error)) {
        console.error('Search failed:', error)
      }
    }
  }

  return (
    <div>
      <input
        type="text"
        value={query}
        onChange={(e) => {
          setQuery(e.target.value)
          search(e.target.value)
        }}
      />
      <ul>
        {results.map((agent) => (
          <li key={agent.id}>{agent.name}</li>
        ))}
      </ul>
    </div>
  )
}
```

## Advanced Patterns

### Request with Custom Headers

```typescript
const agent = await api.agents.get('agent-123', {
  headers: {
    'X-Custom-Header': 'value',
    'X-Request-ID': crypto.randomUUID(),
  },
})
```

### Conditional Requests

```typescript
// Only fetch if ID is present
const agent = id ? await api.agents.get(id) : null

// Fetch with fallback
const agent = await api.agents.get('agent-123').catch(() => null)
```

### Parallel Requests

```typescript
// Fetch multiple resources in parallel
const [agentsResult, tasks, tools] = await Promise.all([
  api.agents.list(),
  api.tasks.list(),
  api.tools.list(),
])

const agents = agentsResult.agents
```

### Sequential Requests with Dependencies

```typescript
// Create agent, then assign tools
const agent = await api.agents.create({
  name: 'My Agent',
  model: 'claude-3-opus',
  tier: 'research',
  system_prompt: 'You are helpful',
})

// Get available tools
const tools = await api.tools.list()

// Assign tools to the agent
await api.agents.setTools(
  agent.id,
  tools.map((t) => t.id)
)
```

### SSE Streaming

```typescript
import { createSSEStream } from '@/api'
import { isHttpError } from '@/api'

const abort = createSSEStream('/stream/endpoint', {
  onEvent: (event) => {
    const parsed = JSON.parse(event.data)
    console.log(`[${event.event}]`, parsed)
  },
  onDone: () => {
    console.log('Stream complete')
  },
  onError: (error) => {
    if (isHttpError(error)) {
      console.error(`HTTP ${error.status}: ${error.statusText}`)
    } else {
      console.error('Stream error:', error.message)
    }
  },
})

// Cancel the stream when done
abort()
```

## Global Configuration

### Configure Defaults on App Startup

```typescript
// In your App.tsx or main entry point
import { configure } from '@/api'

configure({
  timeout: 30000, // 30 seconds
  retries: 2, // Retry failed requests twice
  retryDelay: 1000, // Start with 1 second delay
  requestLogger: (ctx) => {
    if (import.meta.env.DEV) {
      console.log(`-> ${ctx.method} ${ctx.url}`)
    }
  },
  responseLogger: (ctx) => {
    if (import.meta.env.DEV) {
      console.log(`<- ${ctx.status} ${ctx.url}`)
    }
  },
})
```

## Interceptors

### Add Global Error Handler

```typescript
import { addInterceptor } from '@/api'

const removeInterceptor = addInterceptor({
  onError: (error) => {
    if (error.status === 401) {
      window.location.href = '/login'
    }

    if (error.status && error.status >= 500) {
      showToast('Server error. Please try again later.')
    }

    return error
  },
})
```

### Add Request ID to All Requests

```typescript
import { addInterceptor } from '@/api'

addInterceptor({
  onRequest: (ctx) => {
    ctx.config.headers = {
      ...ctx.config.headers,
      'X-Request-ID': crypto.randomUUID(),
    }
    return ctx
  },
})
```

## Testing

### Mock API Calls in Tests

```typescript
import { vi } from 'vitest'
import { api } from '@/api'

// Mock the api
vi.mock('@/api', () => ({
  api: {
    agents: {
      list: vi.fn(),
      get: vi.fn(),
      create: vi.fn(),
    },
  },
}))

// In your test
it('loads agents', async () => {
  const mockAgents = [
    { id: '1', name: 'Agent 1' },
    { id: '2', name: 'Agent 2' },
  ]

  vi.mocked(api.agents.list).mockResolvedValue({
    agents: mockAgents,
  })

  const { result } = renderHook(() => useAgents())

  await waitFor(() => {
    expect(result.current.agents).toEqual(mockAgents)
  })
})
```
