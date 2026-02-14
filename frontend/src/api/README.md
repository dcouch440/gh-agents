# API Client Documentation

State-of-the-art typed API client for the nexor frontend.

## Features

- **Full Type Safety**: Every endpoint is fully typed with request/response types
- **Request Deduplication**: Prevents duplicate in-flight GET requests
- **Retry Logic**: Automatic retry with exponential backoff
- **Request Cancellation**: Built-in AbortController support
- **Interceptors**: Request/response/error interceptors
- **Timeout Support**: Configurable per-request or global timeouts
- **Better Errors**: Discriminated `ApiError` class with type guards for narrowing
- **Request Logging**: Built-in logging support for debugging
- **Immutability**: All exported API objects are deeply frozen at runtime
- **File Uploads**: FormData support (Content-Type header automatically removed)

## Basic Usage

### Using Typed Endpoints (Recommended)

```typescript
import { api } from '@/api'

// List all agents
const { agents } = await api.agents.list()

// Get a specific agent
const agent = await api.agents.get('agent-id')

// Create an agent
const newAgent = await api.agents.create({
  name: 'My Agent',
  model: 'claude-3',
  systemPrompt: 'You are a helpful assistant',
})

// Update an agent
const updated = await api.agents.update('agent-id', {
  name: 'Updated Name',
})

// Delete an agent
await api.agents.delete('agent-id')
```

### Using Low-Level API (For Custom Endpoints)

```typescript
import { api } from '@/api'

// GET request
const data = await api.get<MyResponseType>('/custom/endpoint')

// POST request
const result = await api.post<MyResponseType>('/custom/endpoint', {
  key: 'value',
})

// PATCH request
const updated = await api.patch<MyResponseType>('/custom/endpoint', {
  key: 'new value',
})

// PUT request
const replaced = await api.put<MyResponseType>('/custom/endpoint', {
  key: 'value',
})

// DELETE request
await api.del('/custom/endpoint')
```

## Request Configuration

All endpoint methods accept an optional `RequestConfig` as the last parameter:

```typescript
const agent = await api.agents.get('agent-id', {
  timeout: 5000,          // 5 second timeout
  retries: 3,             // Retry up to 3 times
  retryDelay: 1000,       // Start with 1 second between retries (exponential backoff)
  signal: abortController.signal,  // AbortController for cancellation
  headers: {              // Custom headers
    'X-Custom-Header': 'value',
  },
})
```

## Error Handling

The API client throws typed `ApiError` instances with a discriminated `type` field:

```typescript
import { api, ApiError } from '@/api'

try {
  const agent = await api.agents.get('invalid-id')
} catch (error) {
  if (error instanceof ApiError) {
    console.log(error.type)        // 'http_error', 'network_error', 'timeout_error', etc.
    console.log(error.status)      // HTTP status code (if applicable)
    console.log(error.statusText)  // HTTP status text (if applicable)
    console.log(error.body)        // Response body (if applicable)
    console.log(error.url)         // The URL that failed

    switch (error.type) {
      case 'http_error':
        if (error.status === 404) console.log('Agent not found')
        else if (error.status === 401) console.log('Unauthorized')
        break
      case 'network_error':
        console.log('Network connection failed')
        break
      case 'timeout_error':
        console.log('Request timed out')
        break
      case 'abort_error':
        console.log('Request was cancelled')
        break
    }
  }
}
```

### Type Guards

For cleaner error narrowing without `instanceof` checks:

```typescript
import { isHttpError, isNetworkError, hasStatus, isClientError, isServerError } from '@/api'

try {
  await api.agents.get('agent-id')
} catch (error) {
  if (hasStatus(error, 404)) {
    // error is narrowed to ApiError & { type: 'http_error'; status: number }
    console.log('Not found')
  } else if (isClientError(error)) {
    console.log('Client error:', error)
  } else if (isServerError(error)) {
    console.log('Server error, retrying...')
  } else if (isNetworkError(error)) {
    console.log('Network issue')
  }
}
```

Available guards: `isApiError`, `isHttpError`, `isNetworkError`, `isTimeoutError`, `isAbortError`, `hasStatus`, `isClientError`, `isServerError`.

## Immutability

All exported API objects are frozen at runtime using `Object.freeze()`. This prevents accidental mutation of the API surface:

```typescript
import { api } from '@/api'

// The top-level api object is frozen
Object.isFrozen(api) // true

// Every namespace is frozen
Object.isFrozen(api.agents) // true
Object.isFrozen(api.tasks)  // true

// Attempting to mutate will silently fail (or throw in strict mode)
api.agents.list = () => {} // No effect
```

## Global Configuration

Configure defaults for all requests:

```typescript
import { configure } from '@/api'

configure({
  timeout: 30000,        // Default 30 second timeout
  retries: 2,            // Retry failed requests twice by default
  retryDelay: 1000,      // 1 second base retry delay
  requestLogger: (ctx) => {
    console.log(`→ ${ctx.method} ${ctx.url}`)
  },
  responseLogger: (ctx) => {
    console.log(`← ${ctx.status} ${ctx.url}`)
  },
})
```

## Request Cancellation

Cancel requests using AbortController:

```typescript
const controller = new AbortController()

// Start a request
const promise = api.agents.list({ signal: controller.signal })

// Cancel it
controller.abort()

try {
  await promise
} catch (error) {
  if (isAbortError(error)) {
    console.log('Request was cancelled')
  }
}
```

## Interceptors

Add request/response/error interceptors:

```typescript
import { addInterceptor } from '@/api'

const removeInterceptor = addInterceptor({
  onRequest: (ctx) => {
    ctx.config.headers = {
      ...ctx.config.headers,
      'X-Custom-Header': 'value',
    }
    return ctx
  },
  onResponse: (ctx) => {
    console.log(`Response from ${ctx.status}`)
    return ctx
  },
  onError: (error) => {
    if (error.status === 401) {
      window.location.href = '/login'
    }
    return error
  },
})

// Remove the interceptor when done
removeInterceptor()
```

## Request Deduplication

GET requests are automatically deduplicated. If you make multiple identical GET requests while one is in-flight, they'll all share the same promise:

```typescript
// These three requests will only result in ONE network call
const [agents1, agents2, agents3] = await Promise.all([
  api.agents.list(),
  api.agents.list(),
  api.agents.list(),
])
```

## Retry Logic

Failed requests are automatically retried with exponential backoff:

```typescript
const agent = await api.agents.get('agent-id', {
  retries: 3,
  retryDelay: 1000, // 1s, then 2s, then 4s
})
```

**Retry behavior:**
- Retries are NOT attempted for 4xx errors (client errors)
- Retries ARE attempted for 5xx errors and network errors
- Each retry waits longer: `retryDelay * 2^(attempt - 1)`

## File Uploads

Upload files using FormData (Content-Type header is automatically removed to let the browser set the multipart boundary):

```typescript
const formData = new FormData()
formData.append('file', fileBlob, 'filename.txt')
formData.append('metadata', JSON.stringify({ key: 'value' }))

const result = await api.post<UploadResponse>('/upload', formData)
```

## Cancel All In-Flight Requests

Clear the dedup cache (useful for cleanup on unmount):

```typescript
import { cancelInFlightRequests } from '@/api'

cancelInFlightRequests()
```

## Available Endpoints

All endpoints follow the pattern: `api.<resource>.<method>`

### Auth
- `auth.login(body, config?)`
- `auth.register(body, config?)`
- `auth.me(config?)`

### Agents
- `agents.list(config?)`
- `agents.get(id, config?)`
- `agents.create(body, config?)`
- `agents.update(id, body, config?)`
- `agents.delete(id, config?)`
- `agents.getTools(id, config?)`
- `agents.setTools(id, toolIds, config?)`
- `agents.getContext(id, config?)`
- `agents.setContext(id, docIds, config?)`

### Tasks
- `tasks.list(config?)`
- `tasks.get(id, config?)`
- `tasks.create(body, config?)`
- `tasks.update(id, body, config?)`
- `tasks.delete(id, config?)`

### Tools
- `tools.list(config?)`
- `tools.get(id, config?)`
- `tools.create(body, config?)`
- `tools.update(id, body, config?)`
- `tools.delete(id, config?)`

### Documents
- `documents.list(config?)`
- `documents.get(id, config?)`
- `documents.create(body, config?)`
- `documents.update(id, body, config?)`
- `documents.delete(id, config?)`
- `documents.search(query, config?)`

### Sessions
- `sessions.list(config?)`
- `sessions.get(id, config?)`
- `sessions.create(body, config?)`
- `sessions.update(id, body, config?)`
- `sessions.delete(id, config?)`
- `sessions.chat(id, message, config?)`
- `sessions.getHistory(id, config?)`
- `sessions.clearMessages(id, config?)`

### Chat
- `chat.send(message, config?)`
- `chat.getHistory(config?)`

### Config
- `config.get(config?)`
- `config.update(body, config?)`

### Stats
- `stats.get(config?)`

### Agent Executions
- `agentExecutions.list(params?, config?)`
- `agentExecutions.get(id, config?)`
- `agentExecutions.getMessages(id, config?)`
- `agentExecutions.sendMessage(id, body, config?)`
- `agentExecutions.approve(id, body?, config?)`

### Output Schemas
- `outputSchemas.list(config?)`
- `outputSchemas.get(id, config?)`
- `outputSchemas.create(body, config?)`
- `outputSchemas.update(id, body, config?)`
- `outputSchemas.delete(id, config?)`

### Prompt Templates
- `promptTemplates.list(config?)`
- `promptTemplates.get(id, config?)`
- `promptTemplates.create(body, config?)`
- `promptTemplates.update(id, body, config?)`
- `promptTemplates.delete(id, config?)`

### Costs
- `costs.list(config?)`

### Results
- `results.list(config?)`
- `results.get(id, config?)`

### Workflows
- `workflows.list(config?)`
- `workflows.get(id, config?)`
- `workflows.create(body, config?)`
- `workflows.update(id, body, config?)`
- `workflows.delete(id, config?)`
- `workflows.run(id, body?, config?)`
- `workflows.listExecutions(workflowId, config?)`
- `workflows.listSteps(workflowId, config?)`
- `workflows.createStep(workflowId, body, config?)`
- `workflows.getStep(workflowId, stepId, config?)`
- `workflows.updateStep(workflowId, stepId, body, config?)`
- `workflows.deleteStep(workflowId, stepId, config?)`
- `workflows.listEdges(workflowId, config?)`
- `workflows.createEdge(workflowId, body, config?)`
- `workflows.deleteEdge(workflowId, edgeId, config?)`
- `workflows.listStepDocuments(workflowId, stepId, config?)`
- `workflows.addStepDocument(workflowId, stepId, docId, config?)`
- `workflows.removeStepDocument(workflowId, stepId, docId, config?)`
- `workflows.listDocumentDefs(workflowId, stepId, config?)`
- `workflows.createDocumentDef(workflowId, stepId, body, config?)`
- `workflows.updateDocumentDef(workflowId, stepId, defId, body, config?)`
- `workflows.deleteDocumentDef(workflowId, stepId, defId, config?)`
- `workflows.listRosterAgents(workflowId, stepId, config?)`
- `workflows.createRosterAgent(workflowId, stepId, body, config?)`
- `workflows.deleteRosterAgent(workflowId, stepId, agentId, config?)`
- `workflows.listRoomStepMembers(workflowId, stepId, config?)`
- `workflows.getStepSession(workflowId, stepId, config?)`
- `workflows.getOrCreateStepSession(workflowId, stepId, config?)`
- `workflows.clearStepMessages(workflowId, stepId, config?)`
- `workflows.getStepChatDebug(workflowId, stepId, config?)`

### Context Response
- `contextResponse.get(config?)`

### Modes
- `modes.list(config?)`

### Tool Routers
- `toolRouters.list(config?)`
- `toolRouters.get(id, config?)`
- `toolRouters.create(body, config?)`
- `toolRouters.update(id, body, config?)`
- `toolRouters.delete(id, config?)`
- `toolRouters.getTools(id, config?)`
- `toolRouters.setTools(id, body, config?)`

### Router Modes
- `routerModes.listByRouter(routerId, config?)`
- `routerModes.createForRouter(routerId, body, config?)`
- `routerModes.get(id, config?)`
- `routerModes.update(id, body, config?)`
- `routerModes.delete(id, config?)`
- `routerModes.getTools(id, config?)`
- `routerModes.setTools(id, body, config?)`

### Rooms
- `rooms.get(id, config?)`
- `rooms.create(body, config?)`
- `rooms.update(id, body, config?)`
- `rooms.delete(id, config?)`
- `rooms.listMembers(id, config?)`
- `rooms.addMember(id, body, config?)`
- `rooms.setMembers(id, body, config?)`
- `rooms.removeMember(id, agentId, config?)`
- `rooms.createSession(id, config?)`

### Room Sessions
- `roomSessions.get(id, config?)`
- `roomSessions.sendMessage(id, body, config?)`
- `roomSessions.getTranscript(id, config?)`
- `roomSessions.close(id, config?)`
- `roomSessions.listOutputs(id, config?)`

### Collections
- `collections.list(config?)`
- `collections.get(id, config?)`
- `collections.create(body, config?)`
- `collections.update(id, body, config?)`
- `collections.delete(id, config?)`
- `collections.run(id, config?)`
- `collections.getRunStatus(runId, config?)`

### Protocols
- `protocols.list(config?)`
- `protocols.get(id, config?)`
- `protocols.create(body, config?)`
- `protocols.update(id, body, config?)`
- `protocols.delete(id, config?)`
- `protocols.listTypes(config?)`
- `protocols.createPort(protocolId, body, config?)`
- `protocols.deletePort(protocolId, portId, config?)`
- `protocols.preview(id, config?)`

## SSE (Server-Sent Events)

For streaming responses:

```typescript
import { createSSEStream } from '@/api'

const abort = createSSEStream('/stream/endpoint', {
  onEvent: (event) => {
    console.log(event.event, event.data)
  },
  onDone: () => {
    console.log('Stream complete')
  },
  onError: (error) => {
    // error is ApiError — use type guards for narrowing
    console.error(error.type, error.message)
  },
}, {
  headers: { 'X-Custom': 'value' },
  signal: controller.signal,
})

// Cancel the stream
abort()
```

## Best Practices

1. **Always use typed endpoints** instead of raw `api.get/post/etc`
2. **Use type guards** (`isHttpError`, `hasStatus`, etc.) for clean error handling
3. **Use AbortController** for requests in useEffect hooks
4. **Configure timeouts** for long-running requests
5. **Use interceptors** for cross-cutting concerns (auth, logging, etc.)
6. **Cancel requests on unmount** to prevent memory leaks

```typescript
useEffect(() => {
  const controller = new AbortController()

  const load = async () => {
    try {
      const { agents } = await api.agents.list({
        signal: controller.signal,
      })
      setAgents(agents)
    } catch (error) {
      if (!isAbortError(error)) {
        setError(error instanceof Error ? error.message : 'Unknown error')
      }
    }
  }

  void load()

  return () => {
    controller.abort()
  }
}, [])
```
