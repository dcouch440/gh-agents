import { renderHook, act, waitFor } from '@testing-library/react'
import { useCreateSession, useUpdateSession, useDeleteSession } from './useSessionMutations'
import { mockSession } from '@/test/fixtures'

const { mockPost, mockPatch, mockDel } = vi.hoisted(() => ({
  mockPost: vi.fn(), mockPatch: vi.fn(), mockDel: vi.fn(),
}))

vi.mock('@/api', () => ({ api: { post: mockPost, patch: mockPatch, del: mockDel } }))
describe('useSessionMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useCreateSession', () => {
    it('creates a session and returns it', async () => {
      mockPost.mockResolvedValue(mockSession)
      const { result } = renderHook(() => useCreateSession())

      let session: unknown
      await act(async () => {
        session = await result.current.mutate({ mode_id: 'home', title: 'New session' })
      })

      expect(session).toEqual(mockSession)
      expect(mockPost).toHaveBeenCalledWith('/sessions', { mode_id: 'home', title: 'New session' })
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBe(null)
    })

    it('sets loading during mutation', async () => {
      let resolve: (v: unknown) => void
      mockPost.mockReturnValue(new Promise((r) => { resolve = r }))
      const { result } = renderHook(() => useCreateSession())

      act(() => {
        void result.current.mutate({ mode_id: 'home', title: 'New session' })
      })

      await waitFor(() => {
        expect(result.current.loading).toBe(true)
      })

      act(() => {
        resolve!(mockSession)
      })

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Network error'))
      const { result } = renderHook(() => useCreateSession())

      let caught: unknown
      await act(async () => {
        try {
          await result.current.mutate({ mode_id: 'home', title: 'Fail' })
        } catch (e) {
          caught = e
        }
      })

      expect(caught).toBeInstanceOf(Error)
      expect((caught as Error).message).toBe('Network error')
      expect(result.current.error).toBe('Network error')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useUpdateSession', () => {
    it('updates a session and returns it', async () => {
      const updated = { ...mockSession, title: 'Updated' }
      mockPatch.mockResolvedValue(updated)
      const { result } = renderHook(() => useUpdateSession())

      let session: unknown
      await act(async () => {
        session = await result.current.mutate('session-001', { title: 'Updated' })
      })

      expect(session).toEqual(updated)
      expect(mockPatch).toHaveBeenCalledWith('/sessions/session-001', { title: 'Updated' })
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBe(null)
    })

    it('sets error and throws on failure', async () => {
      mockPatch.mockRejectedValue(new Error('Not found'))
      const { result } = renderHook(() => useUpdateSession())

      let caught: unknown
      await act(async () => {
        try {
          await result.current.mutate('session-001', { title: 'Fail' })
        } catch (e) {
          caught = e
        }
      })

      expect(caught).toBeInstanceOf(Error)
      expect((caught as Error).message).toBe('Not found')
      expect(result.current.error).toBe('Not found')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useDeleteSession', () => {
    it('deletes a session', async () => {
      mockDel.mockResolvedValue(undefined)
      const { result } = renderHook(() => useDeleteSession())

      await act(async () => {
        await result.current.mutate('session-001')
      })

      expect(mockDel).toHaveBeenCalledWith('/sessions/session-001')
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBe(null)
    })

    it('sets error and throws on failure', async () => {
      mockDel.mockRejectedValue(new Error('Forbidden'))
      const { result } = renderHook(() => useDeleteSession())

      let caught: unknown
      await act(async () => {
        try {
          await result.current.mutate('session-001')
        } catch (e) {
          caught = e
        }
      })

      expect(caught).toBeInstanceOf(Error)
      expect((caught as Error).message).toBe('Forbidden')
      expect(result.current.error).toBe('Forbidden')
      expect(result.current.loading).toBe(false)
    })
  })
})
