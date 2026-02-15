// ============================================================================
// executionStore — Hand-written store for agent executions + messages + SSE
// ============================================================================

import { createStore, createNormalizedMap, nmFromArray, nmSet, nmDelete, toArray, nmGet, extractError } from './lib'
import type { NormalizedMap } from './lib'
import { api } from '@/api'
import { createSSEStream } from '@/api/sse'
import { API } from '@/constants'
import type { AgentExecution, ExecutionMessage, ApproveExecutionRequest } from '@/types/execution'

// ── State ────────────────────────────────────────────────────────────────────

type ExecutionState = {
  items: NormalizedMap<AgentExecution>
  messagesByExecution: Record<string, ExecutionMessage[]>
  activeStreams: Record<string, (() => void) | null>
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<ExecutionState>(() => ({
  items: createNormalizedMap<AgentExecution>(),
  messagesByExecution: {},
  activeStreams: {},
  loading: false,
  error: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const EMPTY_MESSAGES: ExecutionMessage[] = []

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: ExecutionState): AgentExecution[] => toArray(s.items)

const selectById =
  (id: string) =>
  (s: ExecutionState): AgentExecution | undefined =>
    nmGet(s.items, id)

const selectMessages =
  (id: string) =>
  (s: ExecutionState): ExecutionMessage[] =>
    s.messagesByExecution[id] ?? EMPTY_MESSAGES

const selectLoading = (s: ExecutionState): boolean => s.loading

const selectError = (s: ExecutionState): string | null => s.error

// ── Async Actions ────────────────────────────────────────────────────────────

const fetchAll = async (params?: { status?: string }): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.agentExecutions.list(params)
    store.setState({ items: nmFromArray(data), loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError('executions', e) })
  }
}

const fetchOne = async (id: string): Promise<AgentExecution> => {
  const execution = await api.agentExecutions.get(id)
  store.setState((s) => ({ items: nmSet(s.items, execution.id, execution) }))
  return execution
}

const fetchMessages = async (id: string): Promise<void> => {
  try {
    const data = await api.agentExecutions.getMessages(id)
    store.setState((s) => ({
      messagesByExecution: { ...s.messagesByExecution, [id]: data.messages },
    }))
  } catch (e) {
    store.setState({ error: extractError('executions', e) })
  }
}

const sendMessage = async (executionId: string, content: string): Promise<void> => {
  store.setState({ error: null })
  try {
    const response = await api.agentExecutions.sendMessage(executionId, { content })

    // Append user message
    store.setState((s) => ({
      messagesByExecution: {
        ...s.messagesByExecution,
        [executionId]: [...(s.messagesByExecution[executionId] ?? []), response.message],
      },
    }))

    // Create temp assistant message for streaming
    const tempId = `streaming-${Date.now()}`
    let accumulated = ''

    const tempMessage: ExecutionMessage = {
      id: tempId,
      agent_execution_id: executionId,
      role: 'assistant',
      content: '',
      tool_call_id: null,
      input_tokens: 0,
      output_tokens: 0,
      created_at: new Date().toISOString(),
    }

    store.setState((s) => ({
      messagesByExecution: {
        ...s.messagesByExecution,
        [executionId]: [...(s.messagesByExecution[executionId] ?? []), tempMessage],
      },
    }))

    // Capture the index of the temp message for O(1) updates during streaming
    const tempIndex = (store.getState().messagesByExecution[executionId] ?? []).length - 1

    // Batch token updates to once per animation frame to avoid per-token array copies
    let pendingFrame: number | null = null

    const flushTokens = (): void => {
      pendingFrame = null
      const current = accumulated
      store.setState((s) => {
        const msgs = s.messagesByExecution[executionId] ?? []
        const updated = msgs.slice()
        updated[tempIndex] = { ...msgs[tempIndex], content: current }
        return {
          messagesByExecution: {
            ...s.messagesByExecution,
            [executionId]: updated,
          },
        }
      })
    }

    // Open SSE stream
    const abort = createSSEStream(API.EXECUTION_MESSAGE_STREAM(executionId, response.stream_id), {
      onEvent: (event) => {
        if (event.event === 'token') {
          const tokenText = JSON.parse(event.data) as string
          accumulated += tokenText
          pendingFrame ??= requestAnimationFrame(flushTokens)
        }
      },
      onDone: () => {
        if (pendingFrame !== null) {
          cancelAnimationFrame(pendingFrame)
          flushTokens()
        }
        store.setState((s) => ({
          activeStreams: { ...s.activeStreams, [executionId]: null },
        }))
        void fetchMessages(executionId)
      },
      onError: (err) => {
        if (pendingFrame !== null) {
          cancelAnimationFrame(pendingFrame)
          flushTokens()
        }
        store.setState((s) => ({
          activeStreams: { ...s.activeStreams, [executionId]: null },
          error: err.message,
        }))
        void fetchMessages(executionId)
      },
    })

    // Wrap abort to also cancel any pending rAF — stopStream() can't
    // reach the closure-scoped pendingFrame directly
    const stop = (): void => {
      if (pendingFrame !== null) {
        cancelAnimationFrame(pendingFrame)
        flushTokens()
      }
      abort()
    }

    store.setState((s) => ({
      activeStreams: { ...s.activeStreams, [executionId]: stop },
    }))
  } catch (e) {
    store.setState({ error: extractError('executions', e) })
  }
}

const stopStream = (executionId: string): void => {
  const { activeStreams } = store.getState()
  const abort = activeStreams[executionId]
  if (abort) {
    abort()
    store.setState((s) => ({
      activeStreams: { ...s.activeStreams, [executionId]: null },
    }))
  }
}

const approve = async (executionId: string, structuredOutput?: Record<string, unknown>): Promise<void> => {
  const body: ApproveExecutionRequest | undefined = structuredOutput ? { structured_output: structuredOutput } : undefined
  await api.agentExecutions.approve(executionId, body)
  await fetchMessages(executionId)
}

// ── Sync Mutations (for WS integration) ─────────────────────────────────────

const upsert = (execution: AgentExecution): void => {
  store.setState((s) => ({ items: nmSet(s.items, execution.id, execution) }))
}

const removeById = (id: string): void => {
  store.setState((s) => ({ items: nmDelete(s.items, id) }))
}

// ── Export ────────────────────────────────────────────────────────────────────

export const executionStore = {
  store,
  selectAll,
  selectById,
  selectMessages,
  selectLoading,
  selectError,
  fetchAll,
  fetchOne,
  fetchMessages,
  sendMessage,
  stopStream,
  approve,
  upsert,
  removeById,
}

export type { ExecutionState }
