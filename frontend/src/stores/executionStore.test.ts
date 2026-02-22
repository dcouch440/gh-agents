import { executionStore } from './executionStore'
import { nmSize, nmGet, createNormalizedMap } from './lib'
import type { AgentExecution, ExecutionMessage } from '@/types/execution'

const { mockList, mockGet, mockGetMessages, mockSendMessage, mockApprove, mockCreateSSEStream } = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockGet: vi.fn(),
  mockGetMessages: vi.fn(),
  mockSendMessage: vi.fn(),
  mockApprove: vi.fn(),
  mockCreateSSEStream: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    agentExecutions: {
      list: mockList,
      get: mockGet,
      getMessages: mockGetMessages,
      sendMessage: mockSendMessage,
      approve: mockApprove,
    },
  },
}))

vi.mock('@/api/sse', () => ({
  createSSEStream: mockCreateSSEStream,
}))

const exec1: AgentExecution = {
  id: 'e1',
  execution_type: 'interactive_review',
  stage_execution_id: 'se1',
  agent_id: 'a1',
  workflow_step_id: null,
  is_interactive: true,
  parent_agent_execution_id: null,
  system_prompt_rendered: 'You are an assistant.',
  input: 'Hello',
  output: null,
  structured_output: null,
  selected_mode_id: null,
  status: 'awaiting_user',
  started_at: '2025-01-01T00:00:00Z',
  completed_at: null,
}

const exec2: AgentExecution = {
  ...exec1,
  id: 'e2',
  status: 'completed',
  output: 'Done',
}

const msg1: ExecutionMessage = {
  id: 'm1',
  agent_execution_id: 'e1',
  role: 'user',
  content: 'Hello',
  tool_call_id: null,
  input_tokens: 10,
  output_tokens: 0,
  created_at: '2025-01-01T00:00:00Z',
}

const msg2: ExecutionMessage = {
  id: 'm2',
  agent_execution_id: 'e1',
  role: 'assistant',
  content: 'Hi there!',
  tool_call_id: null,
  input_tokens: 0,
  output_tokens: 15,
  created_at: '2025-01-01T00:00:01Z',
}

beforeEach(() => {
  vi.clearAllMocks()
  executionStore.store.setState({
    items: createNormalizedMap(),
    messagesByExecution: {},
    activeStreams: {},
    loading: false,
    error: null,
  })
})

describe('executionStore', () => {
  describe('fetchAll', () => {
    it('populates items from api', async () => {
      mockList.mockResolvedValue([exec1, exec2])
      await executionStore.fetchAll()

      const s = executionStore.store.getState()
      expect(nmSize(s.items)).toBe(2)
      expect(nmGet(s.items, 'e1')).toEqual(exec1)
      expect(s.loading).toBe(false)
      expect(s.error).toBeNull()
    })

    it('passes status param to api', async () => {
      mockList.mockResolvedValue([exec1])
      await executionStore.fetchAll({ status: 'awaiting_user' })

      expect(mockList).toHaveBeenCalledWith({ status: 'awaiting_user' })
    })

    it('sets error on failure', async () => {
      mockList.mockRejectedValue(new Error('Network error'))
      await executionStore.fetchAll()

      const s = executionStore.store.getState()
      expect(s.error).toBe('Network error')
      expect(s.loading).toBe(false)
    })
  })

  describe('fetchOne', () => {
    it('upserts single execution', async () => {
      mockGet.mockResolvedValue(exec1)
      const result = await executionStore.fetchOne('e1')

      expect(result).toEqual(exec1)
      expect(nmGet(executionStore.store.getState().items, 'e1')).toEqual(exec1)
    })
  })

  describe('fetchMessages', () => {
    it('stores messages by execution id', async () => {
      mockGetMessages.mockResolvedValue({ messages: [msg1, msg2] })
      await executionStore.fetchMessages('e1')

      const msgs = executionStore.selectMessages('e1')(executionStore.store.getState())
      expect(msgs).toEqual([msg1, msg2])
    })

    it('sets error on failure', async () => {
      mockGetMessages.mockRejectedValue(new Error('Not found'))
      await executionStore.fetchMessages('e1')

      expect(executionStore.store.getState().error).toBe('Not found')
    })
  })

  describe('sendMessage', () => {
    it('appends user message and starts SSE stream', async () => {
      const userMsg: ExecutionMessage = { ...msg1, id: 'sent-1' }
      mockSendMessage.mockResolvedValue({ message: userMsg, stream_id: 'stream-1' })
      mockCreateSSEStream.mockReturnValue(vi.fn())

      await executionStore.sendMessage('e1', 'Hello')

      expect(mockSendMessage).toHaveBeenCalledWith('e1', { content: 'Hello' })
      expect(mockCreateSSEStream).toHaveBeenCalledWith(
        expect.stringContaining('/agent-executions/e1/messages/stream-1/stream') as string,
        expect.objectContaining({
          onEvent: expect.any(Function) as unknown,
          onDone: expect.any(Function) as unknown,
          onError: expect.any(Function) as unknown,
        }) as unknown,
      )

      // User message + temp assistant message should be in store
      const msgs = executionStore.selectMessages('e1')(executionStore.store.getState())
      expect(msgs).toHaveLength(2)
      expect(msgs[0]).toEqual(userMsg)
      expect(msgs[1].role).toBe('assistant')
      expect(msgs[1].content).toBe('')
    })

    it('stores abort function in activeStreams', async () => {
      const mockAbort = vi.fn()
      mockSendMessage.mockResolvedValue({ message: msg1, stream_id: 's1' })
      mockCreateSSEStream.mockReturnValue(mockAbort)

      await executionStore.sendMessage('e1', 'Hi')

      const storedFn = executionStore.store.getState().activeStreams['e1']
      expect(typeof storedFn).toBe('function')
      // Calling the stored stop function should invoke the underlying abort
      storedFn?.()
      expect(mockAbort).toHaveBeenCalled()
    })
  })

  describe('stopStream', () => {
    it('calls abort and clears stream', () => {
      const mockAbort = vi.fn()
      executionStore.store.setState({ activeStreams: { e1: mockAbort } })

      executionStore.stopStream('e1')

      expect(mockAbort).toHaveBeenCalledOnce()
      expect(executionStore.store.getState().activeStreams['e1']).toBeNull()
    })

    it('does nothing when no active stream', () => {
      executionStore.stopStream('e1')
      // No error thrown
    })
  })

  describe('approve', () => {
    it('calls approve API and refetches messages', async () => {
      mockApprove.mockResolvedValue(undefined)
      mockGetMessages.mockResolvedValue({ messages: [msg1] })

      await executionStore.approve('e1')

      expect(mockApprove).toHaveBeenCalledWith('e1', undefined)
      expect(mockGetMessages).toHaveBeenCalledWith('e1')
    })

    it('passes structured output when provided', async () => {
      mockApprove.mockResolvedValue(undefined)
      mockGetMessages.mockResolvedValue({ messages: [] })

      await executionStore.approve('e1', { result: 'ok' })

      expect(mockApprove).toHaveBeenCalledWith('e1', { structured_output: { result: 'ok' } })
    })
  })

  describe('sync mutations', () => {
    it('upsert adds execution', () => {
      executionStore.upsert(exec1)

      expect(nmGet(executionStore.store.getState().items, 'e1')).toEqual(exec1)
    })

    it('removeById removes execution', () => {
      executionStore.upsert(exec1)
      executionStore.removeById('e1')

      expect(nmGet(executionStore.store.getState().items, 'e1')).toBeUndefined()
    })
  })

  describe('selectors', () => {
    it('selectAll returns array', async () => {
      mockList.mockResolvedValue([exec1, exec2])
      await executionStore.fetchAll()

      expect(executionStore.selectAll(executionStore.store.getState())).toHaveLength(2)
    })

    it('selectById returns undefined for missing', () => {
      const result = executionStore.selectById('missing')(executionStore.store.getState())
      expect(result).toBeUndefined()
    })

    it('selectMessages returns empty array for missing', () => {
      const result = executionStore.selectMessages('missing')(executionStore.store.getState())
      expect(result).toEqual([])
    })
  })
})
