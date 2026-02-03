import { renderHook, act } from '@testing-library/react'
import { useCreateTool, useUpdateTool, useDeleteTool } from './useToolMutations'
import { mockTool } from '@/test/fixtures'

const { mockPost, mockPatch, mockDel } = vi.hoisted(() => ({
  mockPost: vi.fn(),
  mockPatch: vi.fn(),
  mockDel: vi.fn(),
}))

vi.mock('@/api', () => ({ api: { post: mockPost, patch: mockPatch, del: mockDel } }))
describe('useToolMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useCreateTool', () => {
    it('creates a tool and returns it', async () => {
      mockPost.mockResolvedValue(mockTool)
      const { result } = renderHook(() => useCreateTool())

      let tool: unknown
      await act(async () => {
        tool = await result.current.mutate({ name: 'search_files', description: 'Search', category: 'codebase', parameter_schema: {}, output_schema: {}, enabled: true, is_builtin: true, cluster_id: null })
      })

      expect(tool).toEqual(mockTool)
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Create failed'))
      const { result } = renderHook(() => useCreateTool())

      await act(async () => {
        await expect(result.current.mutate({ name: 'search_files', description: 'Search', category: 'codebase', parameter_schema: {}, output_schema: {}, enabled: true, is_builtin: true, cluster_id: null })).rejects.toThrow('Create failed')
      })

      expect(result.current.error).toBe('Create failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useUpdateTool', () => {
    it('updates a tool and returns it', async () => {
      mockPatch.mockResolvedValue(mockTool)
      const { result } = renderHook(() => useUpdateTool())

      let tool: unknown
      await act(async () => {
        tool = await result.current.mutate('tool-001', { name: 'updated_tool' })
      })

      expect(tool).toEqual(mockTool)
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPatch.mockRejectedValue(new Error('Update failed'))
      const { result } = renderHook(() => useUpdateTool())

      await act(async () => {
        await expect(result.current.mutate('tool-001', { name: 'updated_tool' })).rejects.toThrow('Update failed')
      })

      expect(result.current.error).toBe('Update failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useDeleteTool', () => {
    it('deletes a tool', async () => {
      mockDel.mockResolvedValue(undefined)
      const { result } = renderHook(() => useDeleteTool())

      await act(async () => {
        await result.current.mutate('tool-001')
      })

      expect(mockDel).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockDel.mockRejectedValue(new Error('Delete failed'))
      const { result } = renderHook(() => useDeleteTool())

      await act(async () => {
        await expect(result.current.mutate('tool-001')).rejects.toThrow('Delete failed')
      })

      expect(result.current.error).toBe('Delete failed')
      expect(result.current.loading).toBe(false)
    })
  })
})
