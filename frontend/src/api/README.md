# API Client Documentation

State-of-the-art typed API client for the nexor frontend.

## Features

- **Full Type Safety**: Every endpoint is fully typed with request/response types
- **Request Deduplication**: Prevents duplicate in-flight GET requests
- **Retry Logic**: Automatic retry with exponential backoff
- **Request Cancellation**: Built-in AbortController support
- **Interceptors**: Request/response/error interceptors
- **Timeout Support**: Configurable per-request or global timeouts
- **Better Errors**: Typed error classes with detailed context
- **Request Logging**: Built-in logging support for debugging
- **File Uploads**: FormData support with progress tracking

## Basic Usage

### Using Typed Endpoints (Recommended)

```typescript
import { endpoints } from '@/api'

// List all agents
const { agents } = await endpoints.agents.list()

// Get a specific agent
const agent = await endpoints.agents.get('agent-id')

// Create an agent
const newAgent = await endpoints.agents.create({
  name: 'My Agent',
  model: 'claude-3',
  systemPrompt: 'You are a helpful assistant',
})

// Update an agent
const updated = await endpoints.agents.update('agent-id', {
  name: 'Updated Name',
})

// Delete an agent
await endpoints.agents.delete('agent-id')
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
const agent = await endpoints.agents.get('agent-id', {
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

The API client throws typed `ApiError` instances:

```typescript
import { endpoints, ApiError } from '@/api'

try {
  const agent = await endpoints.agents.get('invalid-id')
} catch (error) {
  if (error instanceof ApiError) {
    console.log(error.type)        // 'http_error', 'network_error', 'timeout_error', etc.
    console.log(error.status)      // HTTP status code (if applicable)
    console.log(error.statusText)  // HTTP status text (if applicable)
    console.log(error.body)        // Response body (if applicable)
    console.log(error.url)         // The URL that failed

    // Handle specific error types
    switch (error.type) {
      case 'http_error':
        if (error.status === 404) {
          console.log('Agent not found')
        } else if (error.status === 401) {
          console.log('Unauthorized')
        }
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
const promise = endpoints.agents.list({ signal: controller.signal })

// Cancel it
controller.abort()

try {
  await promise
} catch (error) {
  if (error instanceof ApiError && error.type === 'abort_error') {
    console.log('Request was cancelled')
  }
}
```

## Interceptors

Add request/response/error interceptors:

```typescript
import { addInterceptor } from '@/api'

// Add auth token to all requests
const removeInterceptor = addInterceptor({
  onRequest: (ctx) => {
    // Modify request before it's sent
    ctx.config.headers = {
      ...ctx.config.headers,
      'X-Custom-Header': 'value',
    }
    return ctx
  },
  onResponse: (ctx) => {
    // Transform response data
    console.log(`Response from ${ctx.status}`)
    return ctx
  },
  onError: (error) => {
    // Handle or transform errors
    if (error.status === 401) {
      // Redirect to login
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
  endpoints.agents.list(),
  endpoints.agents.list(),
  endpoints.agents.list(),
])

// All three will receive the same data
console.log(agents1 === agents2) // true
```

## Retry Logic

Failed requests are automatically retried with exponential backoff:

```typescript
// This will retry up to 3 times with exponential backoff
const agent = await endpoints.agents.get('agent-id', {
  retries: 3,
  retryDelay: 1000, // 1s, then 2s, then 4s
})
```

**Retry behavior:**
- Retries are NOT attempted for 4xx errors (client errors)
- Retries ARE attempted for 5xx errors and network errors
- Each retry waits longer: `retryDelay * 2^(attempt - 1)`

## File Uploads

Upload files using FormData:

```typescript
const formData = new FormData()
formData.append('file', fileBlob, 'filename.txt')
formData.append('metadata', JSON.stringify({ key: 'value' }))

const result = await api.post<UploadResponse>('/upload', formData, {
  headers: {
    // Content-Type will be automatically set for FormData
  },
  onUploadProgress: (progress) => {
    console.log(`Upload progress: ${progress * 100}%`)
  },
})
```

## Cancel All In-Flight Requests

Cancel all pending requests (useful for cleanup on unmount):

```typescript
import { cancelInFlightRequests } from '@/api'

// Cancel everything
cancelInFlightRequests()
```

## Available Endpoints

All endpoints follow the pattern: `endpoints.<resource>.<method>`

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
- `sessions.delete(id, config?)`
- `sessions.chat(id, message, config?)`
- `sessions.getHistory(id, config?)`

### Chat
- `chat.send(message, config?)`
- `chat.getHistory(config?)`

### Config
- `config.get(config?)`
- `config.update(body, config?)`

### Stats
- `stats.get(config?)`

### Pipelines
- `pipelines.list(config?)`
- `pipelines.get(id, config?)`
- `pipelines.create(body, config?)`
- `pipelines.update(id, body, config?)`
- `pipelines.delete(id, config?)`
- `pipelines.renderStage(id, stage, config?)`
- `pipelines.getSideTasks(id, stage, config?)`
- `pipelines.getSideTask(id, stage, taskId, config?)`

### Pipeline Runs
- `pipelineRuns.list(config?)`
- `pipelineRuns.get(id, config?)`
- `pipelineRuns.approve(id, config?)`
- `pipelineRuns.getTree(runId, config?)`

### Stage Members
- `stageMembers.list(pipelineId, stageNum, config?)`
- `stageMembers.create(pipelineId, stageNum, body, config?)`
- `stageMembers.delete(pipelineId, stageNum, memberId, config?)`

### Agent Executions
- `agentExecutions.get(id, config?)`
- `agentExecutions.getMessages(id, config?)`
- `agentExecutions.approve(id, config?)`

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
- `workflows.listSteps(workflowId, config?)`
- `workflows.createStep(workflowId, body, config?)`
- `workflows.getStep(workflowId, stepId, config?)`
- `workflows.updateStep(workflowId, stepId, body, config?)`
- `workflows.deleteStep(workflowId, stepId, config?)`
- `workflows.listEdges(workflowId, config?)`
- `workflows.createEdge(workflowId, body, config?)`
- `workflows.listStepDocuments(workflowId, stepId, config?)`
- `workflows.addStepDocument(workflowId, stepId, docId, config?)`
- `workflows.removeStepDocument(workflowId, stepId, docId, config?)`

## Migration Guide

### Before (old way):

```typescript
import { api } from '@/api'
import { API } from '@/constants'

const { agents } = await api.get<AgentsResponse>(API.AGENTS)
```

### After (new way):

```typescript
import { endpoints } from '@/api'

const { agents } = await endpoints.agents.list()
```

The new way provides:
- Better type inference
- Less boilerplate
- Cleaner code
- IDE autocomplete for all endpoints
- Consistent API surface

## Best Practices

1. **Always use typed endpoints** instead of raw `api.get/post/etc`
2. **Handle errors explicitly** using try/catch
3. **Use AbortController** for requests in useEffect hooks
4. **Configure timeouts** for long-running requests
5. **Use interceptors** for cross-cutting concerns (auth, logging, etc.)
6. **Cancel requests on unmount** to prevent memory leaks

```typescript
useEffect(() => {
  const controller = new AbortController()

  const load = async () => {
    try {
      const { agents } = await endpoints.agents.list({
        signal: controller.signal,
      })
      setAgents(agents)
    } catch (error) {
      if (error instanceof ApiError && error.type !== 'abort_error') {
        setError(error.message)
      }
    }
  }

  void load()

  return () => {
    controller.abort()
  }
}, [])
```
