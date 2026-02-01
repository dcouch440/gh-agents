import { renderHook, act } from '@testing-library/react'
import { useSendMessage, useSendSessionMessage, useClearHistory } from './useChatMutations'

const { mockPost, mockDel, mockCreateSSE } = vi.hoisted(() => ({
  mockPost: vi.fn(),
  mockDel: vi.fn(),
  mockCreateSSE: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: { post: mockPost, del: mockDel },
  createSSEStream: mockCreateSSE,
}))

vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})

describe('useChatMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useSendMessage', () => {
    it('calls POST /chat and returns message_id', async () => {
      mockPost.mockResolvedValue({ message_id: 'msg-123', status: 'ok' })
      const { result } = renderHook(() => useSendMessage())

      let messageId: string | undefined
      await act(async () => {
        messageId = await result.current.send({ content: 'hello' })
      })

      expect(messageId).toBe('msg-123')
      expect(mockPost).toHaveBeenCalledWith('/chat', { content: 'hello' })
      expect(result.current.error).toBeNull()
      expect(result.current.loading).toBe(false)
    })

    it('calls createSSEStream when onEvent is provided', async () => {
      mockPost.mockResolvedValue({ message_id: 'msg-456', status: 'ok' })
      mockCreateSSE.mockReturnValue(() => undefined)
      const { result } = renderHook(() => useSendMessage())

      const onEvent = vi.fn()
      const onDone = vi.fn()

      await act(async () => {
        await result.current.send({ content: 'hello' }, onEvent, onDone)
      })

      expect(mockCreateSSE).toHaveBeenCalledWith(
        '/chat/msg-456/stream',
        expect.objectContaining({
          onEvent,
        }),
      )
      expect(result.current.streaming).toBe(true)
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Network error'))
      const { result } = renderHook(() => useSendMessage())

      await act(async () => {
        await expect(result.current.send({ content: 'hello' })).rejects.toThrow('Network error')
      })

      expect(result.current.error).toBe('Network error')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useSendSessionMessage', () => {
    it('calls POST /sessions/{id}/chat and returns message_id', async () => {
      mockPost.mockResolvedValue({ message_id: 'msg-789', status: 'ok' })
      mockCreateSSE.mockReturnValue(() => undefined)
      const { result } = renderHook(() => useSendSessionMessage())

      let messageId: string | undefined
      await act(async () => {
        messageId = await result.current.send('session-001', { content: 'hi' })
      })

      expect(messageId).toBe('msg-789')
      expect(mockPost).toHaveBeenCalledWith('/sessions/session-001/chat', { content: 'hi' })
    })
  })

  describe('useClearHistory', () => {
    it('calls DELETE /chat/history', async () => {
      mockDel.mockResolvedValue(undefined)
      const { result } = renderHook(() => useClearHistory())

      await act(async () => {
        await result.current.mutate()
      })

      expect(mockDel).toHaveBeenCalledWith('/chat/history')
      expect(result.current.error).toBeNull()
      expect(result.current.loading).toBe(false)
    })

    it('sets error and throws on failure', async () => {
      mockDel.mockRejectedValue(new Error('Server error'))
      const { result } = renderHook(() => useClearHistory())

      await act(async () => {
        await expect(result.current.mutate()).rejects.toThrow('Server error')
      })

      expect(result.current.error).toBe('Server error')
      expect(result.current.loading).toBe(false)
    })
  })
})
