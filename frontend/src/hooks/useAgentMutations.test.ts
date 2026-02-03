import { renderHook, act } from '@testing-library/react'
import { useCreateAgent, useUpdateAgent, useDeleteAgent, useAgentTools, useAgentContextDocs } from './useAgentMutations'
import { mockAgent, mockTool } from '@/test/fixtures'

const { mockGet, mockPost, mockPatch, mockDel, mockPut } = vi.hoisted(() => ({
  mockGet: vi.fn(),
  mockPost: vi.fn(),
  mockPatch: vi.fn(),
  mockDel: vi.fn(),
  mockPut: vi.fn(),
}))

vi.mock('@/api', () => ({ api: { get: mockGet, post: mockPost, patch: mockPatch, del: mockDel, put: mockPut } }))
describe('useAgentMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useCreateAgent', () => {
    it('creates an agent and returns it', async () => {
      mockPost.mockResolvedValue(mockAgent)
      const { result } = renderHook(() => useCreateAgent())

      let agent: unknown
      await act(async () => {
        agent = await result.current.mutate({ name: 'TestBot', model_provider: 'anthropic', model_id: 'claude-sonnet-4-20250514', model_max_tokens: 8192, model_temperature: 0.7 })
      })

      expect(agent).toEqual(mockAgent)
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Create failed'))
      const { result } = renderHook(() => useCreateAgent())

      await act(async () => {
        await expect(result.current.mutate({ name: 'TestBot', model_provider: 'anthropic', model_id: 'claude-sonnet-4-20250514', model_max_tokens: 8192, model_temperature: 0.7 })).rejects.toThrow('Create failed')
      })

      expect(result.current.error).toBe('Create failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useUpdateAgent', () => {
    it('updates an agent and returns it', async () => {
      mockPatch.mockResolvedValue(mockAgent)
      const { result } = renderHook(() => useUpdateAgent())

      let agent: unknown
      await act(async () => {
        agent = await result.current.mutate('agent-001', { name: 'Updated' })
      })

      expect(agent).toEqual(mockAgent)
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPatch.mockRejectedValue(new Error('Update failed'))
      const { result } = renderHook(() => useUpdateAgent())

      await act(async () => {
        await expect(result.current.mutate('agent-001', { name: 'Updated' })).rejects.toThrow('Update failed')
      })

      expect(result.current.error).toBe('Update failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useDeleteAgent', () => {
    it('deletes an agent', async () => {
      mockDel.mockResolvedValue(undefined)
      const { result } = renderHook(() => useDeleteAgent())

      await act(async () => {
        await result.current.mutate('agent-001')
      })

      expect(mockDel).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockDel.mockRejectedValue(new Error('Delete failed'))
      const { result } = renderHook(() => useDeleteAgent())

      await act(async () => {
        await expect(result.current.mutate('agent-001')).rejects.toThrow('Delete failed')
      })

      expect(result.current.error).toBe('Delete failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useAgentTools', () => {
    it('loads tools for an agent', async () => {
      mockGet.mockResolvedValue({ agent_id: 'agent-001', tools: [mockTool] })
      const { result } = renderHook(() => useAgentTools())

      await act(async () => {
        await result.current.load('agent-001')
      })

      expect(result.current.tools).toEqual([mockTool])
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on load failure', async () => {
      mockGet.mockRejectedValue(new Error('Load failed'))
      const { result } = renderHook(() => useAgentTools())

      await act(async () => {
        await expect(result.current.load('agent-001')).rejects.toThrow('Load failed')
      })

      expect(result.current.error).toBe('Load failed')
      expect(result.current.loading).toBe(false)
    })

    it('saves tools for an agent', async () => {
      mockPut.mockResolvedValue(undefined)
      const { result } = renderHook(() => useAgentTools())

      await act(async () => {
        await result.current.save('agent-001', ['tool-001'])
      })

      expect(mockPut).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on save failure', async () => {
      mockPut.mockRejectedValue(new Error('Save failed'))
      const { result } = renderHook(() => useAgentTools())

      await act(async () => {
        await expect(result.current.save('agent-001', ['tool-001'])).rejects.toThrow('Save failed')
      })

      expect(result.current.error).toBe('Save failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useAgentContextDocs', () => {
    it('loads context docs for an agent', async () => {
      const mockDoc = { id: 'doc-001', title: 'Test', summary: 'A doc', ref_tag: 'test', tags: ['test'], doc_type: 'note', updated_at: '2025-01-01T00:00:00Z' }
      mockGet.mockResolvedValue({ agent_id: 'agent-001', documents: [mockDoc] })
      const { result } = renderHook(() => useAgentContextDocs())

      await act(async () => {
        await result.current.load('agent-001')
      })

      expect(result.current.docs).toEqual([mockDoc])
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on load failure', async () => {
      mockGet.mockRejectedValue(new Error('Load failed'))
      const { result } = renderHook(() => useAgentContextDocs())

      await act(async () => {
        await expect(result.current.load('agent-001')).rejects.toThrow('Load failed')
      })

      expect(result.current.error).toBe('Load failed')
      expect(result.current.loading).toBe(false)
    })

    it('saves context docs for an agent', async () => {
      mockPut.mockResolvedValue(undefined)
      const { result } = renderHook(() => useAgentContextDocs())

      await act(async () => {
        await result.current.save('agent-001', ['doc-001'])
      })

      expect(mockPut).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on save failure', async () => {
      mockPut.mockRejectedValue(new Error('Save failed'))
      const { result } = renderHook(() => useAgentContextDocs())

      await act(async () => {
        await expect(result.current.save('agent-001', ['doc-001'])).rejects.toThrow('Save failed')
      })

      expect(result.current.error).toBe('Save failed')
      expect(result.current.loading).toBe(false)
    })
  })
})
