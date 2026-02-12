import { renderHook, act, waitFor } from '@testing-library/react'
import { useAssistantSession, reducer, initialState } from './useAssistantSession'
import type { AssistantState, AssistantAction } from './useAssistantSession'
import type { Session, ChatMessage } from '@/types'

const { mockGetStepSession, mockGetOrCreate, mockGetHistory, mockClearMessages, mockSend, mockAbort } = vi.hoisted(() => ({
  mockGetStepSession: vi.fn(),
  mockGetOrCreate: vi.fn(),
  mockGetHistory: vi.fn(),
  mockClearMessages: vi.fn(),
  mockSend: vi.fn(),
  mockAbort: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    workflows: {
      getStepSession: mockGetStepSession,
      getOrCreateStepSession: mockGetOrCreate,
      clearStepMessages: mockClearMessages,
    },
    sessions: {
      getHistory: mockGetHistory,
    },
  },
  createSSEStream: vi.fn(),
}))

vi.mock('./useChatMutations', () => ({
  useSendSessionMessage: () => ({
    send: mockSend,
    abort: mockAbort,
    loading: false,
    streaming: false,
    error: null,
  }),
}))

const makeSession = (id = 'session-001'): Session => ({
  id,
  mode_id: 'step_chat',
  agent_id: null,
  draft_config: null,
  title: 'Test Chat',
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
})

const makeHistory = (): ChatMessage[] => [
  { id: 'msg-1', role: 'user', content: 'hello', timestamp: '2025-01-01T00:00:00Z' },
  { id: 'msg-2', role: 'assistant', content: 'hi there', timestamp: '2025-01-01T00:00:01Z' },
]

// Helper to apply a sequence of actions to the reducer
const applyActions = (actions: AssistantAction[], state: AssistantState = initialState): AssistantState =>
  actions.reduce((s, a) => reducer(s, a), state)

describe('useAssistantSession', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns empty state when no existing session (GET 404)', async () => {
    mockGetStepSession.mockRejectedValue(new Error('404 Not Found'))

    const { result } = renderHook(() => useAssistantSession('wf-001', 'step-001'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(result.current.messages).toEqual([])
    expect(result.current.streamingSegments).toEqual([])
    expect(result.current.error).toBeNull()
  })

  it('loads existing session and history on mount', async () => {
    const session = makeSession()
    mockGetStepSession.mockResolvedValue(session)
    mockGetHistory.mockResolvedValue(makeHistory())

    const { result } = renderHook(() => useAssistantSession('wf-001', 'step-001'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(result.current.messages).toHaveLength(2)
    expect(result.current.messages[0]?.content).toBe('hello')
    expect(result.current.messages[1]?.content).toBe('hi there')
  })

  it('sendMessage creates session on first call when no session exists', async () => {
    mockGetStepSession.mockRejectedValue(new Error('404 Not Found'))
    const session = makeSession()
    mockGetOrCreate.mockResolvedValue(session)
    mockSend.mockResolvedValue('msg-new')

    const { result } = renderHook(() => useAssistantSession('wf-001', 'step-001'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    act(() => {
      result.current.sendMessage('set up docs')
    })

    // Optimistically appended user + assistant placeholders
    expect(result.current.messages).toHaveLength(2)
    expect(result.current.messages[0]?.role).toBe('user')
    expect(result.current.messages[0]?.content).toBe('set up docs')

    await waitFor(() => {
      expect(mockGetOrCreate).toHaveBeenCalledWith('wf-001', 'step-001')
    })

    await waitFor(() => {
      expect(mockSend).toHaveBeenCalledWith(
        'session-001',
        { message: 'set up docs' },
        expect.any(Function),
        expect.any(Function),
        expect.any(Function),
      )
    })
  })

  it('sendMessage skips session creation when session already exists', async () => {
    const session = makeSession()
    mockGetStepSession.mockResolvedValue(session)
    mockGetHistory.mockResolvedValue([])
    mockSend.mockResolvedValue('msg-new')

    const { result } = renderHook(() => useAssistantSession('wf-001', 'step-001'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    act(() => {
      result.current.sendMessage('hello')
    })

    await waitFor(() => {
      expect(mockSend).toHaveBeenCalled()
    })

    expect(mockGetOrCreate).not.toHaveBeenCalled()
  })

  it('clearHistory calls API and clears local messages', async () => {
    const session = makeSession()
    mockGetStepSession.mockResolvedValue(session)
    mockGetHistory.mockResolvedValue(makeHistory())
    mockClearMessages.mockResolvedValue(undefined)

    const { result } = renderHook(() => useAssistantSession('wf-001', 'step-001'))

    await waitFor(() => {
      expect(result.current.messages).toHaveLength(2)
    })

    act(() => {
      result.current.clearHistory()
    })

    expect(result.current.messages).toEqual([])

    await waitFor(() => {
      expect(mockClearMessages).toHaveBeenCalledWith('wf-001', 'step-001')
    })
  })

  it('returns null workflowId as empty state', async () => {
    const { result } = renderHook(() => useAssistantSession(null, 'step-001'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(result.current.messages).toEqual([])
    expect(mockGetStepSession).not.toHaveBeenCalled()
  })

  it('resets state when stepId changes', async () => {
    const session = makeSession()
    mockGetStepSession.mockResolvedValue(session)
    mockGetHistory.mockResolvedValue(makeHistory())

    const { result, rerender } = renderHook(
      ({ stepId }: { stepId: string }) => useAssistantSession('wf-001', stepId),
      { initialProps: { stepId: 'step-001' } },
    )

    await waitFor(() => {
      expect(result.current.messages).toHaveLength(2)
    })

    mockGetStepSession.mockRejectedValue(new Error('404 Not Found'))

    rerender({ stepId: 'step-002' })

    await waitFor(() => {
      expect(result.current.messages).toEqual([])
    })
  })
})

describe('reducer', () => {
  const baseState: AssistantState = {
    ...initialState,
    isLoading: false,
    messages: [
      { id: 'u1', role: 'user', content: 'hello' },
      { id: 'a1', role: 'assistant', content: '' },
    ],
    streamingSegments: [],
  }

  describe('STREAM_TOKEN', () => {
    it('creates a new text segment when segments are empty', () => {
      const state = reducer(baseState, { type: 'STREAM_TOKEN', text: 'Hello' })

      expect(state.streamingSegments).toEqual([{ type: 'text', content: 'Hello' }])
    })

    it('appends to existing text segment', () => {
      const withText = applyActions(
        [
          { type: 'STREAM_TOKEN', text: 'Hello' },
          { type: 'STREAM_TOKEN', text: ' world' },
        ],
        baseState,
      )

      expect(withText.streamingSegments).toEqual([{ type: 'text', content: 'Hello world' }])
    })

    it('creates new text segment after tool segment', () => {
      const state = applyActions(
        [
          { type: 'STREAM_TOKEN', text: 'Before' },
          { type: 'STREAM_TOOL_START', toolId: 't1', toolName: 'think' },
          { type: 'STREAM_TOKEN', text: 'After' },
        ],
        baseState,
      )

      expect(state.streamingSegments).toHaveLength(3)
      expect(state.streamingSegments[0]).toEqual({ type: 'text', content: 'Before' })
      expect(state.streamingSegments[1]).toEqual({
        type: 'tool',
        toolId: 't1',
        toolName: 'think',
        status: 'running',
      })
      expect(state.streamingSegments[2]).toEqual({ type: 'text', content: 'After' })
    })

    it('updates last assistant message content', () => {
      const state = applyActions(
        [
          { type: 'STREAM_TOKEN', text: 'Hello' },
          { type: 'STREAM_TOKEN', text: ' world' },
        ],
        baseState,
      )

      const lastMsg = state.messages[state.messages.length - 1]
      expect(lastMsg?.content).toBe('Hello world')
    })
  })

  describe('STREAM_TOOL_START', () => {
    it('adds a running tool segment', () => {
      const state = reducer(baseState, {
        type: 'STREAM_TOOL_START',
        toolId: 't1',
        toolName: 'create_doc_def',
      })

      expect(state.streamingSegments).toEqual([
        { type: 'tool', toolId: 't1', toolName: 'create_doc_def', status: 'running' },
      ])
    })
  })

  describe('STREAM_TOOL_END', () => {
    it('updates matching tool segment to complete', () => {
      const state = applyActions(
        [
          { type: 'STREAM_TOOL_START', toolId: 't1', toolName: 'create_doc_def' },
          { type: 'STREAM_TOOL_END', toolId: 't1' },
        ],
        baseState,
      )

      expect(state.streamingSegments).toEqual([
        { type: 'tool', toolId: 't1', toolName: 'create_doc_def', status: 'complete' },
      ])
    })

    it('only updates the matching tool by id', () => {
      const state = applyActions(
        [
          { type: 'STREAM_TOOL_START', toolId: 't1', toolName: 'create_doc_def' },
          { type: 'STREAM_TOOL_START', toolId: 't2', toolName: 'read_context' },
          { type: 'STREAM_TOOL_END', toolId: 't1' },
        ],
        baseState,
      )

      expect(state.streamingSegments[0]).toEqual(
        expect.objectContaining({ toolId: 't1', status: 'complete' }),
      )
      expect(state.streamingSegments[1]).toEqual(
        expect.objectContaining({ toolId: 't2', status: 'running' }),
      )
    })
  })

  describe('STREAM_DOC_UPDATE', () => {
    it('adds a doc_update segment', () => {
      const state = reducer(baseState, {
        type: 'STREAM_DOC_UPDATE',
        docId: 'd1',
        title: 'API Reference',
      })

      expect(state.streamingSegments).toEqual([
        { type: 'doc_update', docId: 'd1', title: 'API Reference' },
      ])
    })
  })

  describe('STREAM_FINALIZE', () => {
    it('clears streaming segments', () => {
      const state = applyActions(
        [
          { type: 'STREAM_TOKEN', text: 'Final content' },
          { type: 'STREAM_TOOL_START', toolId: 't1', toolName: 'think' },
          { type: 'STREAM_FINALIZE' },
        ],
        baseState,
      )

      expect(state.streamingSegments).toEqual([])
    })

    it('preserves message content after finalization', () => {
      const state = applyActions(
        [
          { type: 'STREAM_TOKEN', text: 'Complete response' },
          { type: 'STREAM_FINALIZE' },
        ],
        baseState,
      )

      const lastMsg = state.messages[state.messages.length - 1]
      expect(lastMsg?.content).toBe('Complete response')
    })
  })

  describe('STREAM_ERROR', () => {
    it('clears streaming segments', () => {
      const state = applyActions(
        [
          { type: 'STREAM_TOKEN', text: 'Partial' },
          { type: 'STREAM_ERROR', error: 'Connection lost' },
        ],
        baseState,
      )

      expect(state.streamingSegments).toEqual([])
    })

    it('sets error on state', () => {
      const state = reducer(baseState, { type: 'STREAM_ERROR', error: 'Connection lost' })
      expect(state.error).toBe('Connection lost')
    })

    it('sets error text on empty assistant message', () => {
      const state = reducer(baseState, { type: 'STREAM_ERROR', error: 'Timeout' })

      const lastMsg = state.messages[state.messages.length - 1]
      expect(lastMsg?.content).toBe('Error: Timeout')
    })

    it('preserves existing content on assistant message', () => {
      const withContent = reducer(baseState, { type: 'STREAM_TOKEN', text: 'Partial response' })
      const state = reducer(withContent, { type: 'STREAM_ERROR', error: 'Timeout' })

      const lastMsg = state.messages[state.messages.length - 1]
      expect(lastMsg?.content).toBe('Partial response')
    })
  })

  describe('RESET', () => {
    it('clears streaming segments along with everything else', () => {
      const withStreaming = applyActions(
        [
          { type: 'STREAM_TOKEN', text: 'text' },
          { type: 'STREAM_TOOL_START', toolId: 't1', toolName: 'think' },
        ],
        baseState,
      )

      const state = reducer(withStreaming, { type: 'RESET' })
      expect(state.streamingSegments).toEqual([])
      expect(state.messages).toEqual([])
    })
  })

  describe('CLEAR_MESSAGES', () => {
    it('clears streaming segments along with messages', () => {
      const withStreaming = reducer(baseState, { type: 'STREAM_TOKEN', text: 'text' })
      const state = reducer(withStreaming, { type: 'CLEAR_MESSAGES' })

      expect(state.streamingSegments).toEqual([])
      expect(state.messages).toEqual([])
    })
  })

  describe('full streaming sequence', () => {
    it('handles a complete streaming lifecycle with tools', () => {
      const state = applyActions(
        [
          { type: 'STREAM_TOKEN', text: "I'll create docs.\n\n" },
          { type: 'STREAM_TOOL_START', toolId: 't1', toolName: 'create_doc_def' },
          { type: 'STREAM_TOOL_END', toolId: 't1' },
          { type: 'STREAM_DOC_UPDATE', docId: 'd1', title: 'API Reference' },
          { type: 'STREAM_TOKEN', text: '\n\nCreated the document.' },
          { type: 'STREAM_FINALIZE' },
        ],
        baseState,
      )

      // Segments cleared after finalization
      expect(state.streamingSegments).toEqual([])

      // Message has the accumulated text
      const lastMsg = state.messages[state.messages.length - 1]
      expect(lastMsg?.content).toBe("I'll create docs.\n\n\n\nCreated the document.")
    })
  })
})
