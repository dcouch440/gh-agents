import { renderHook, waitFor } from '@testing-library/react'
import { useSessions, useChatHistory, useModes } from './useSessions'
import { mockSession, mockChatMessage, mockMode } from '@/test/fixtures'

const { mockSessionsList, mockGetHistory, mockModesList } = vi.hoisted(() => ({
  mockSessionsList: vi.fn(),
  mockGetHistory: vi.fn(),
  mockModesList: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    sessions: { list: mockSessionsList, getHistory: mockGetHistory },
    modes: { list: mockModesList },
  },
}))
describe('useSessions', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useSessions', () => {
    it('fetches and returns sessions', async () => {
      mockSessionsList.mockResolvedValue({ items: [mockSession] })
      const { result } = renderHook(() => useSessions())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.sessions).toEqual([mockSession])
    })

    it('sets error on failure', async () => {
      mockSessionsList.mockRejectedValue(new Error('Failed'))
      const { result } = renderHook(() => useSessions())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.error).toBe('Failed')
    })
  })

  describe('useChatHistory', () => {
    it('fetches messages for a session', async () => {
      mockGetHistory.mockResolvedValue({ messages: [mockChatMessage] })
      const { result } = renderHook(() => useChatHistory('session-001'))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.messages).toEqual([mockChatMessage])
      expect(mockGetHistory).toHaveBeenCalledWith('session-001')
    })

    it('returns empty when sessionId is null', async () => {
      const { result } = renderHook(() => useChatHistory(null))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.messages).toEqual([])
      expect(mockGetHistory).not.toHaveBeenCalled()
    })
  })

  describe('useModes', () => {
    it('fetches and returns modes', async () => {
      mockModesList.mockResolvedValue([mockMode])
      const { result } = renderHook(() => useModes())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.modes).toEqual([mockMode])
    })
  })
})
