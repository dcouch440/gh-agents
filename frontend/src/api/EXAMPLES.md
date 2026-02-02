# API Client Usage Examples

Real-world examples of using the new API client.

## Basic CRUD Operations

### Listing Resources

```typescript
import { endpoints } from '@/api'

// List all agents
const { agents } = await endpoints.agents.list()

// List with custom config
const { agents } = await endpoints.agents.list({
  timeout: 5000,
  retries: 2,
})
```

### Getting a Single Resource

```typescript
// Get a specific agent
const agent = await endpoints.agents.get('agent-123')

// With custom timeout
const agent = await endpoints.agents.get('agent-123', {
  timeout: 10000,
})
```

### Creating Resources

```typescript
// Create an agent
const newAgent = await endpoints.agents.create({
  name: 'My Agent',
  model: 'claude-3-opus',
  tier: 'research',
  system_prompt: 'You are a helpful assistant',
})

// Create a task
const task = await endpoints.tasks.create({
  title: 'Implement feature X',
  description: 'Add authentication to the API',
  priority: 'high',
  tier: 'dev',
})
```

### Updating Resources

```typescript
// Update an agent
const updated = await endpoints.agents.update('agent-123', {
  name: 'Updated Name',
  system_prompt: 'New prompt',
})

// Update a session
await endpoints.sessions.update('session-456', {
  title: 'Updated Session Title',
})
```

### Deleting Resources

```typescript
// Delete an agent
await endpoints.agents.delete('agent-123')

// Delete a document
await endpoints.documents.delete('doc-789')
```

## Error Handling Patterns

### Basic Error Handling

```typescript
import { endpoints, ApiError } from '@/api'

try {
  const agent = await endpoints.agents.get('invalid-id')
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

### Handling Specific Error Types

```typescript
import { endpoints, ApiError } from '@/api'

try {
  const agent = await endpoints.agents.get('agent-123')
} catch (error) {
  if (error instanceof ApiError) {
    switch (error.type) {
      case 'http_error':
        if (error.status === 404) {
          alert('Agent not found')
        } else if (error.status === 401) {
          window.location.href = '/login'
        } else if (error.status === 403) {
          alert('You do not have permission to view this agent')
        }
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
const agent = await endpoints.agents.get('agent-123', {
  retries: 3,
  retryDelay: 1000, // Start with 1s, then 2s, then 4s (exponential backoff)
})
```

## React Hooks Integration

### Basic Data Fetching Hook

```typescript
import { useState, useEffect } from 'react'
import { endpoints, ApiError } from '@/api'
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
        const { agents } = await endpoints.agents.list({
          signal: controller.signal,
        })
        setAgents(agents)
      } catch (err) {
        if (err instanceof ApiError && err.type !== 'abort_error') {
          setError(err.message)
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
import { endpoints, ApiError } from '@/api'
import type { Agent, CreateAgentRequest } from '@/types'

function useCreateAgent() {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const create = useCallback(async (body: CreateAgentRequest): Promise<Agent | null> => {
    setLoading(true)
    setError(null)
    try {
      const agent = await endpoints.agents.create(body)
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
useEffect(() => {
  const controller = new AbortController()

  const fetchData = async () => {
    try {
      const { agents } = await endpoints.agents.list({
        signal: controller.signal,
      })
      setAgents(agents)
    } catch (error) {
      if (error instanceof ApiError && error.type === 'abort_error') {
        console.log('Request cancelled')
      } else {
        console.error('Error:', error)
      }
    }
  }

  void fetchData()

  // Cleanup: cancel request if component unmounts
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
      const { agents } = await endpoints.agents.list({
        signal: controller.signal,
      })
      // Filter results locally (or use a search endpoint)
      const filtered = agents.filter((a) => a.name.includes(searchQuery))
      setResults(filtered)
    } catch (error) {
      if (error instanceof ApiError && error.type !== 'abort_error') {
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
const agent = await endpoints.agents.get('agent-123', {
  headers: {
    'X-Custom-Header': 'value',
    'X-Request-ID': crypto.randomUUID(),
  },
})
```

### Conditional Requests

```typescript
// Only fetch if ID is present
const agent = id ? await endpoints.agents.get(id) : null

// Fetch with fallback
const agent = await endpoints.agents.get('agent-123').catch(() => null)
```

### Parallel Requests

```typescript
// Fetch multiple resources in parallel
const [agentsResult, tasksResult, toolsResult] = await Promise.all([
  endpoints.agents.list(),
  endpoints.tasks.list(),
  endpoints.tools.list(),
])

const agents = agentsResult.agents
const tasks = tasksResult.items
const tools = toolsResult.items
```

### Sequential Requests with Dependencies

```typescript
// Create agent, then assign tools
const agent = await endpoints.agents.create({
  name: 'My Agent',
  model: 'claude-3-opus',
  tier: 'research',
  system_prompt: 'You are helpful',
})

// Get available tools
const { items: tools } = await endpoints.tools.list()

// Assign tools to the agent
await endpoints.agents.setTools(
  agent.id,
  tools.map((t) => t.id)
)
```

### Polling for Updates

```typescript
function usePipelineRun(runId: string) {
  const [run, setRun] = useState<PipelineRun | null>(null)

  useEffect(() => {
    let cancelled = false

    const poll = async () => {
      while (!cancelled) {
        try {
          const latestRun = await endpoints.pipelineRuns.get(runId)
          if (!cancelled) {
            setRun(latestRun)

            // Stop polling if run is complete
            if (latestRun.status === 'completed' || latestRun.status === 'failed') {
              break
            }
          }
        } catch (error) {
          console.error('Polling error:', error)
        }

        // Wait 2 seconds before next poll
        await new Promise((resolve) => setTimeout(resolve, 2000))
      }
    }

    void poll()

    return () => {
      cancelled = true
    }
  }, [runId])

  return run
}
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
      console.log(`→ ${ctx.method} ${ctx.url}`)
    }
  },
  responseLogger: (ctx) => {
    if (import.meta.env.DEV) {
      console.log(`← ${ctx.status} ${ctx.url}`)
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
    // Redirect to login on 401
    if (error.status === 401) {
      window.location.href = '/login'
    }

    // Show toast notification
    if (error.status && error.status >= 500) {
      showToast('Server error. Please try again later.')
    }

    return error
  },
})

// Remove when no longer needed
// removeInterceptor()
```

### Add Request ID to All Requests

```typescript
import { addInterceptor } from '@/api'

addInterceptor({
  onRequest: (ctx) => {
    return {
      ...ctx,
      config: {
        ...ctx.config,
        headers: {
          ...ctx.config.headers,
          'X-Request-ID': crypto.randomUUID(),
        },
      },
    }
  },
})
```

### Log All Responses

```typescript
import { addInterceptor } from '@/api'

addInterceptor({
  onResponse: (ctx) => {
    console.log(`Response from ${ctx.url}:`, {
      status: ctx.status,
      data: ctx.data,
    })
    return ctx
  },
})
```

## Testing

### Mock API Calls in Tests

```typescript
import { vi } from 'vitest'
import { endpoints } from '@/api'

// Mock the endpoints
vi.mock('@/api', () => ({
  endpoints: {
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

  vi.mocked(endpoints.agents.list).mockResolvedValue({
    agents: mockAgents,
  })

  const { result } = renderHook(() => useAgents())

  await waitFor(() => {
    expect(result.current.agents).toEqual(mockAgents)
  })
})
```
