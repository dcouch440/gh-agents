import { renderHook, act, waitFor } from '@testing-library/react'
import { useAssistantSession } from './useAssistantSession'
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
