import { renderHook, waitFor } from '@testing-library/react'
import { useAgents, useAgent } from './useAgents'
import { mockAgent } from '@/test/fixtures'

const { mockList, mockGet } = vi.hoisted(() => ({ mockList: vi.fn(), mockGet: vi.fn() }))

vi.mock('@/api', () => ({ api: { agents: { list: mockList, get: mockGet } } }))
describe('useAgents', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useAgents', () => {
    it('fetches and returns agents', async () => {
      mockList.mockResolvedValue({ agents: [mockAgent] })
      const { result } = renderHook(() => useAgents())

      expect(result.current.loading).toBe(true)

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.agents).toEqual([mockAgent])
      expect(result.current.error).toBeNull()
    })

    it('sets error on failure', async () => {
      mockList.mockRejectedValue(new Error('Network error'))
      const { result } = renderHook(() => useAgents())

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.agents).toEqual([])
      expect(result.current.error).toBe('Network error')
    })
  })

  describe('useAgent', () => {
    it('fetches a single agent by id', async () => {
      mockGet.mockResolvedValue(mockAgent)
      const { result } = renderHook(() => useAgent('agent-001'))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.agent).toEqual(mockAgent)
      expect(result.current.error).toBeNull()
    })

    it('returns null when id is null', async () => {
      const { result } = renderHook(() => useAgent(null))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.agent).toBeNull()
      expect(mockGet).not.toHaveBeenCalled()
    })

    it('sets error on failure', async () => {
      mockGet.mockRejectedValue(new Error('Not found'))
      const { result } = renderHook(() => useAgent('bad-id'))

      await waitFor(() => {
        expect(result.current.loading).toBe(false)
      })

      expect(result.current.error).toBe('Not found')
    })
  })
})
