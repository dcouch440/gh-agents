import { renderHook, waitFor } from '@testing-library/react'
import { useToolRouterMutations } from './useToolRouterMutations'
import { mockToolRouter, mockTool } from '@/test/fixtures'

const { mockCreate, mockUpdate, mockDelete, mockGetTools, mockSetTools } = vi.hoisted(() => ({
  mockCreate: vi.fn(),
  mockUpdate: vi.fn(),
  mockDelete: vi.fn(),
  mockGetTools: vi.fn(),
  mockSetTools: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    toolRouters: {
      create: mockCreate,
      update: mockUpdate,
      delete: mockDelete,
      getTools: mockGetTools,
      setTools: mockSetTools,
    },
  },
}))

describe('useToolRouterMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('createRouter', () => {
    it('creates a router successfully', async () => {
      mockCreate.mockResolvedValue(mockToolRouter)
      const { result } = renderHook(() => useToolRouterMutations())

      expect(result.current.creating).toBe(false)

      const router = await result.current.createRouter({
        name: 'Test Router',
        system_prompt: 'You are a router.',
        model_id: 'claude-sonnet-4-20250514',
      })

      expect(router).toEqual(mockToolRouter)
      expect(result.current.creating).toBe(false)
    })

    it('handles create error', async () => {
      mockCreate.mockRejectedValue(new Error('Create failed'))
      const { result } = renderHook(() => useToolRouterMutations())

      await expect(
        result.current.createRouter({
          name: 'Test',
          system_prompt: 'prompt',
          model_id: 'model',
        }),
      ).rejects.toThrow('Create failed')

      await waitFor(() => expect(result.current.creating).toBe(false))
    })
  })

  describe('updateRouter', () => {
    it('updates a router successfully', async () => {
      const updated = { ...mockToolRouter, name: 'Updated' }
      mockUpdate.mockResolvedValue(updated)
      const { result } = renderHook(() => useToolRouterMutations())

      const router = await result.current.updateRouter('router-001', { name: 'Updated' })

      expect(router).toEqual(updated)
      expect(result.current.updating).toBe(false)
    })

    it('handles update error', async () => {
      mockUpdate.mockRejectedValue(new Error('Update failed'))
      const { result } = renderHook(() => useToolRouterMutations())

      await expect(
        result.current.updateRouter('router-001', { name: 'Updated' }),
      ).rejects.toThrow('Update failed')

      await waitFor(() => expect(result.current.updating).toBe(false))
    })
  })

  describe('deleteRouter', () => {
    it('deletes a router successfully', async () => {
      mockDelete.mockResolvedValue(undefined)
      const { result } = renderHook(() => useToolRouterMutations())

      await result.current.deleteRouter('router-001')

      expect(result.current.deleting).toBe(false)
      expect(mockDelete).toHaveBeenCalledWith('router-001')
    })

    it('handles delete error', async () => {
      mockDelete.mockRejectedValue(new Error('Delete failed'))
      const { result } = renderHook(() => useToolRouterMutations())

      await expect(result.current.deleteRouter('router-001')).rejects.toThrow('Delete failed')

      await waitFor(() => expect(result.current.deleting).toBe(false))
    })
  })

  describe('loadRouterTools', () => {
    it('loads tools successfully', async () => {
      mockGetTools.mockResolvedValue([mockTool])
      const { result } = renderHook(() => useToolRouterMutations())

      const tools = await result.current.loadRouterTools('router-001')

      expect(tools).toEqual([mockTool])
      expect(result.current.loadingTools).toBe(false)
      expect(result.current.toolsError).toBeNull()
    })

    it('handles load error', async () => {
      mockGetTools.mockRejectedValue(new Error('Load failed'))
      const { result } = renderHook(() => useToolRouterMutations())

      await expect(result.current.loadRouterTools('router-001')).rejects.toThrow('Load failed')

      await waitFor(() => {
        expect(result.current.loadingTools).toBe(false)
        expect(result.current.toolsError).toBe('Load failed')
      })
    })
  })

  describe('saveRouterTools', () => {
    it('saves tools successfully', async () => {
      mockSetTools.mockResolvedValue(undefined)
      const { result } = renderHook(() => useToolRouterMutations())

      await result.current.saveRouterTools('router-001', { tool_ids: ['tool-001'] })

      expect(result.current.savingTools).toBe(false)
      expect(result.current.toolsError).toBeNull()
    })

    it('handles save error', async () => {
      mockSetTools.mockRejectedValue(new Error('Save failed'))
      const { result } = renderHook(() => useToolRouterMutations())

      await expect(
        result.current.saveRouterTools('router-001', { tool_ids: ['tool-001'] }),
      ).rejects.toThrow('Save failed')

      await waitFor(() => {
        expect(result.current.savingTools).toBe(false)
        expect(result.current.toolsError).toBe('Save failed')
      })
    })
  })
})
