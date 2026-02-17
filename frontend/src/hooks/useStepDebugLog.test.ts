import { renderHook, act, waitFor } from '@testing-library/react'
import { useStepDebugLog } from './useStepDebugLog'

const { mockGetStepChatDebug, mockSelectActiveWorkflowId } = vi.hoisted(() => ({
  mockGetStepChatDebug: vi.fn(),
  mockSelectActiveWorkflowId: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    workflows: { getStepChatDebug: mockGetStepChatDebug },
  },
}))

vi.mock('@/stores', () => ({
  useStore: (_store: unknown, selector: unknown) => {
    if (selector === mockSelectActiveWorkflowId) return mockSelectActiveWorkflowId()
    return null
  },
  workflowStore: {
    store: {},
    selectActiveWorkflowId: mockSelectActiveWorkflowId,
  },
}))

describe('useStepDebugLog', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSelectActiveWorkflowId.mockReturnValue('wf-1')
  })

  it('fetches debug data on mount and returns messages', async () => {
    mockGetStepChatDebug.mockResolvedValue({
      system_prompt: 'You are helpful.',
      messages: [
        { role: 'user', content: 'Hello' },
        { role: 'assistant', content: 'Hi there' },
      ],
    })

    const { result } = renderHook(() => useStepDebugLog('step-1'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(result.current.error).toBeNull()
    expect(result.current.messages).toHaveLength(3)
    expect(result.current.messages[0]).toEqual({
      id: 'system-prompt',
      role: 'system',
      content: 'You are helpful.',
    })
    expect(result.current.messages[1]).toEqual({
      id: 'debug-msg-0',
      role: 'user',
      content: 'Hello',
    })
    expect(result.current.messages[2]).toEqual({
      id: 'debug-msg-1',
      role: 'assistant',
      content: 'Hi there',
    })
    expect(mockGetStepChatDebug).toHaveBeenCalledWith('wf-1', 'step-1')
  })

  it('sets error on API failure', async () => {
    mockGetStepChatDebug.mockRejectedValue(new Error('Network error'))

    const { result } = renderHook(() => useStepDebugLog('step-1'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(result.current.error).toBe('Network error')
    expect(result.current.messages).toHaveLength(0)
  })

  it('refresh triggers re-fetch', async () => {
    mockGetStepChatDebug.mockResolvedValue({
      system_prompt: 'Prompt',
      messages: [],
    })

    const { result } = renderHook(() => useStepDebugLog('step-1'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(mockGetStepChatDebug).toHaveBeenCalledTimes(1)

    act(() => {
      result.current.refresh()
    })

    await waitFor(() => {
      expect(mockGetStepChatDebug).toHaveBeenCalledTimes(2)
    })
  })

  it('does not fetch when workflowId is null', async () => {
    mockSelectActiveWorkflowId.mockReturnValue(null)

    const { result } = renderHook(() => useStepDebugLog('step-1'))

    // Give time for any async effects
    await new Promise((r) => setTimeout(r, 50))

    expect(mockGetStepChatDebug).not.toHaveBeenCalled()
    expect(result.current.isLoading).toBe(true)
    expect(result.current.messages).toHaveLength(0)
  })
})
