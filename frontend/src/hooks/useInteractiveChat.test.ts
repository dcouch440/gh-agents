import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor, act } from '@testing-library/react'
import { useInteractiveChat } from './useInteractiveChat'
import { mockExecutionMessage } from '@/test/fixtures'

const mockGet = vi.hoisted(() => vi.fn())
const mockPost = vi.hoisted(() => vi.fn())

vi.mock('@/api', () => ({
  api: {
    get: mockGet,
    post: mockPost,
    patch: vi.fn(),
    put: vi.fn(),
    del: vi.fn(),
  },
}))

describe('useInteractiveChat', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('fetches messages on mount', async () => {
    mockGet.mockResolvedValueOnce([mockExecutionMessage])

    const { result } = renderHook(() => useInteractiveChat('agent-exec-001'))

    expect(result.current.loading).toBe(true)

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.messages).toEqual([mockExecutionMessage])
    expect(result.current.error).toBeNull()
  })

  it('sendMessage posts and refetches', async () => {
    mockGet.mockResolvedValueOnce([mockExecutionMessage])

    const { result } = renderHook(() => useInteractiveChat('agent-exec-001'))

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    const updatedMessages = [
      mockExecutionMessage,
      { ...mockExecutionMessage, id: 'msg-002', role: 'assistant' as const, content: 'OK' },
    ]
    mockPost.mockResolvedValueOnce(undefined)
    mockGet.mockResolvedValueOnce(updatedMessages)

    await act(async () => {
      await result.current.sendMessage('Hello')
    })

    expect(mockPost).toHaveBeenCalledWith(
      '/agent-executions/agent-exec-001/messages',
      { content: 'Hello', role: 'user' },
    )
    expect(result.current.messages).toEqual(updatedMessages)
  })

  it('approve posts and refetches', async () => {
    mockGet.mockResolvedValueOnce([mockExecutionMessage])

    const { result } = renderHook(() => useInteractiveChat('agent-exec-001'))

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    mockPost.mockResolvedValueOnce(undefined)
    mockGet.mockResolvedValueOnce([mockExecutionMessage])

    await act(async () => {
      await result.current.approve({ status: 'approved' })
    })

    expect(mockPost).toHaveBeenCalledWith(
      '/agent-executions/agent-exec-001/approve',
      { structured_output: { status: 'approved' } },
    )
  })

  it('sets error on fetch failure', async () => {
    mockGet.mockRejectedValueOnce(new Error('Network error'))

    const { result } = renderHook(() => useInteractiveChat('agent-exec-001'))

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    expect(result.current.error).toBe('Network error')
  })
})
