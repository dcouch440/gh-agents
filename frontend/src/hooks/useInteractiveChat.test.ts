import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor, act } from '@testing-library/react'
import { useInteractiveChat } from './useInteractiveChat'
import { mockExecutionMessage } from '@/test/fixtures'

const mockGet = vi.hoisted(() => vi.fn())
const mockSendMessage = vi.hoisted(() => vi.fn())
const mockApprove = vi.hoisted(() => vi.fn())
const mockCreateSSEStream = vi.hoisted(() => vi.fn())

vi.mock('@/api', () => ({
  api: {
    get: mockGet,
    agentExecutions: {
      sendMessage: mockSendMessage,
      approve: mockApprove,
    },
  },
}))

vi.mock('@/api/sse', () => ({
  createSSEStream: mockCreateSSEStream,
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

    const userMessage = {
      ...mockExecutionMessage,
      id: 'msg-response',
      role: 'user' as const,
      content: 'Hello',
    }

    mockSendMessage.mockResolvedValueOnce({
      message: userMessage,
      stream_id: 'stream-001',
    })

    // Mock SSE stream: immediately invoke onDone so refetch triggers
    const updatedMessages = [
      mockExecutionMessage,
      { ...mockExecutionMessage, id: 'msg-002', role: 'assistant' as const, content: 'OK' },
    ]
    mockCreateSSEStream.mockImplementation((_path: string, callbacks: { onDone: () => void }) => {
      // Simulate stream completing immediately
      setTimeout(() => { callbacks.onDone() }, 0)
      return () => {}
    })
    mockGet.mockResolvedValueOnce(updatedMessages)

    await act(async () => {
      await result.current.sendMessage('Hello')
    })

    expect(mockSendMessage).toHaveBeenCalledWith(
      'agent-exec-001',
      { content: 'Hello' },
    )

    // Wait for SSE onDone to trigger refetch
    await waitFor(() => {
      expect(result.current.messages).toEqual(updatedMessages)
    })
  })

  it('approve posts and refetches', async () => {
    mockGet.mockResolvedValueOnce([mockExecutionMessage])

    const { result } = renderHook(() => useInteractiveChat('agent-exec-001'))

    await waitFor(() => {
      expect(result.current.loading).toBe(false)
    })

    mockApprove.mockResolvedValueOnce(undefined)
    mockGet.mockResolvedValueOnce([mockExecutionMessage])

    await act(async () => {
      await result.current.approve({ status: 'approved' })
    })

    expect(mockApprove).toHaveBeenCalledWith(
      'agent-exec-001',
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
