import { renderHook, act } from '@testing-library/react'
import { useCreateWorkflow, useDeleteWorkflow } from './useWorkflowMutations'
import { mockWorkflow } from '@/test/fixtures'

const { mockPost, mockDel } = vi.hoisted(() => ({
  mockPost: vi.fn(),
  mockDel: vi.fn(),
}))

const mockReload = vi.fn()
const mockLoadWorkflow = vi.fn()

vi.mock('@/api', () => ({ api: { post: mockPost, del: mockDel } }))
vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})
vi.mock('@/hooks/useWorkflowContext', () => ({
  useWorkflowContext: () => ({ reload: mockReload, loadWorkflow: mockLoadWorkflow }),
}))

describe('useWorkflowMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useCreateWorkflow', () => {
    it('creates a workflow and calls reload', async () => {
      mockPost.mockResolvedValue(mockWorkflow)
      const { result } = renderHook(() => useCreateWorkflow())

      let workflow: unknown
      await act(async () => {
        workflow = await result.current.mutate({ name: 'Test Workflow', description: 'A test workflow' })
      })

      expect(workflow).toEqual(mockWorkflow)
      expect(mockPost).toHaveBeenCalledWith('/workflows', { name: 'Test Workflow', description: 'A test workflow' })
      expect(mockReload).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Create failed'))
      const { result } = renderHook(() => useCreateWorkflow())

      await act(async () => {
        await expect(result.current.mutate({ name: 'Test' })).rejects.toThrow('Create failed')
      })

      expect(result.current.error).toBe('Create failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useDeleteWorkflow', () => {
    it('deletes a workflow and calls reload', async () => {
      mockDel.mockResolvedValue(undefined)
      const { result } = renderHook(() => useDeleteWorkflow())

      await act(async () => {
        await result.current.mutate('workflow-001')
      })

      expect(mockDel).toHaveBeenCalledWith('/workflows/workflow-001')
      expect(mockReload).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockDel.mockRejectedValue(new Error('Delete failed'))
      const { result } = renderHook(() => useDeleteWorkflow())

      await act(async () => {
        await expect(result.current.mutate('workflow-001')).rejects.toThrow('Delete failed')
      })

      expect(result.current.error).toBe('Delete failed')
      expect(result.current.loading).toBe(false)
    })
  })
})
