import { renderHook, act } from '@testing-library/react'
import { useSendSessionMessage } from './useChatMutations'

const { mockPost, mockCreateSSE } = vi.hoisted(() => ({
  mockPost: vi.fn(),
  mockCreateSSE: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: { post: mockPost },
  createSSEStream: mockCreateSSE,
}))

describe('useChatMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
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
})
