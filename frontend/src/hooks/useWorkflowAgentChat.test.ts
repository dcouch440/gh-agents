import { renderHook, act, waitFor } from '@testing-library/react'
import { useWorkflowAgentChat } from './useWorkflowAgentChat'
import type { SSECallbacks } from '@/api'

const { mockGetOrCreateAgentSession, mockGetHistory, mockCancelChat, mockPost, mockCreateSSE } = vi.hoisted(() => ({
  mockGetOrCreateAgentSession: vi.fn(),
  mockGetHistory: vi.fn(),
  mockCancelChat: vi.fn(),
  mockPost: vi.fn(),
  mockCreateSSE: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    workflows: { getOrCreateAgentSession: mockGetOrCreateAgentSession },
    sessions: { getHistory: mockGetHistory, cancelChat: mockCancelChat },
    post: mockPost,
  },
  createSSEStream: mockCreateSSE,
}))

/** Last set of SSE callbacks handed to createSSEStream. */
const lastCallbacks = (): SSECallbacks => mockCreateSSE.mock.calls.at(-1)?.[1] as SSECallbacks

const readyChat = async () => {
  const hook = renderHook(() => useWorkflowAgentChat('wf-001'))
  await waitFor(() => {
    expect(mockGetHistory).toHaveBeenCalled()
  })
  act(() => {
    hook.result.current.sendMessage('build me a workflow')
  })
  await waitFor(() => {
    expect(mockCreateSSE).toHaveBeenCalled()
  })
  return hook
}

describe('useWorkflowAgentChat', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetOrCreateAgentSession.mockResolvedValue({ session_id: 'session-001' })
    mockGetHistory.mockResolvedValue([])
    mockPost.mockResolvedValue({ message_id: 'msg-001', status: 'ok' })
    mockCreateSSE.mockReturnValue(vi.fn())
  })

  it('marks the turn as streaming while waiting on the first token', async () => {
    const { result } = await readyChat()

    expect(result.current.streaming).toBe(true)
    // The placeholder the UI hangs the "working" state on.
    expect(result.current.messages.at(-1)).toMatchObject({ role: 'assistant', content: '' })
  })

  it('stops streaming when the user cancels, since an aborted stream never completes', async () => {
    const { result } = await readyChat()

    act(() => {
      result.current.cancelChat()
    })

    expect(result.current.streaming).toBe(false)
    // The never-answered placeholder is dropped rather than left blank.
    expect(result.current.messages.map((m) => m.role)).toEqual(['user'])
  })

  it('keeps whatever streamed before the cancel', async () => {
    const { result } = await readyChat()

    act(() => {
      lastCallbacks().onEvent({ event: 'token', data: 'partial' })
    })
    act(() => {
      result.current.cancelChat()
    })

    await waitFor(() => {
      expect(result.current.messages.at(-1)).toMatchObject({ role: 'assistant', content: 'partial' })
    })
    expect(result.current.streaming).toBe(false)
  })

  it('ends the turn and records the failure when the stream errors', async () => {
    const { result } = await readyChat()

    act(() => {
      lastCallbacks().onError(new Error('Stream transport error') as never)
    })

    expect(result.current.streaming).toBe(false)
    expect(result.current.messages).toEqual([
      expect.objectContaining({ role: 'user', error: 'Stream transport error' }),
    ])
  })
})
