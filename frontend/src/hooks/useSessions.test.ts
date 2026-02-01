import { renderHook, waitFor } from '@testing-library/react'
import { useSessions, useChatHistory, useModes } from './useSessions'
import { mockSession, mockChatMessage, mockMode } from '../test/fixtures'

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('../api', () => ({ api: { get: mockGet } }))
vi.mock('../constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('../constants')
  return { ...actual, USE_MOCK_DATA: false }
})

describe('useSessions', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useSessions', () => {
    it('fetches and returns sessions', async () => {
      mockGet.mockResolvedValue([mockSession])
      const { result } = renderHook(() => useSessions())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.sessions).toEqual([mockSession])
    })

    it('sets error on failure', async () => {
      mockGet.mockRejectedValue(new Error('Failed'))
      const { result } = renderHook(() => useSessions())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.error).toBe('Failed')
    })
  })

  describe('useChatHistory', () => {
    it('fetches messages for a session', async () => {
      mockGet.mockResolvedValue([mockChatMessage])
      const { result } = renderHook(() => useChatHistory('session-001'))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.messages).toEqual([mockChatMessage])
      expect(mockGet).toHaveBeenCalledWith('/sessions/session-001/history')
    })

    it('returns empty when sessionId is null', async () => {
      const { result } = renderHook(() => useChatHistory(null))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.messages).toEqual([])
      expect(mockGet).not.toHaveBeenCalled()
    })
  })

  describe('useModes', () => {
    it('fetches and returns modes', async () => {
      mockGet.mockResolvedValue([mockMode])
      const { result } = renderHook(() => useModes())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.modes).toEqual([mockMode])
    })
  })
})
