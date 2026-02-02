import { renderHook, act } from '@testing-library/react'
import { useCreatePromptTemplate, useUpdatePromptTemplate, useDeletePromptTemplate } from './usePromptTemplateMutations'
import { mockPromptTemplate } from '@/test/fixtures'

const { mockPost, mockPut, mockDel } = vi.hoisted(() => ({
  mockPost: vi.fn(),
  mockPut: vi.fn(),
  mockDel: vi.fn(),
}))

vi.mock('@/api', () => ({ api: { post: mockPost, put: mockPut, del: mockDel } }))
vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})

describe('usePromptTemplateMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useCreatePromptTemplate', () => {
    it('creates a prompt template and returns it', async () => {
      mockPost.mockResolvedValue(mockPromptTemplate)
      const { result } = renderHook(() => useCreatePromptTemplate())

      let template: unknown
      await act(async () => {
        template = await result.current.mutate({ name: 'Test Template', template: 'Hello {{name}}' })
      })

      expect(template).toEqual(mockPromptTemplate)
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Create failed'))
      const { result } = renderHook(() => useCreatePromptTemplate())

      await act(async () => {
        await expect(result.current.mutate({ name: 'Test', template: 'Hello' })).rejects.toThrow('Create failed')
      })

      expect(result.current.error).toBe('Create failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useUpdatePromptTemplate', () => {
    it('updates a prompt template and returns it', async () => {
      mockPut.mockResolvedValue(mockPromptTemplate)
      const { result } = renderHook(() => useUpdatePromptTemplate())

      let template: unknown
      await act(async () => {
        template = await result.current.mutate('template-001', { name: 'Updated' })
      })

      expect(template).toEqual(mockPromptTemplate)
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPut.mockRejectedValue(new Error('Update failed'))
      const { result } = renderHook(() => useUpdatePromptTemplate())

      await act(async () => {
        await expect(result.current.mutate('template-001', { name: 'Updated' })).rejects.toThrow('Update failed')
      })

      expect(result.current.error).toBe('Update failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useDeletePromptTemplate', () => {
    it('deletes a prompt template', async () => {
      mockDel.mockResolvedValue(undefined)
      const { result } = renderHook(() => useDeletePromptTemplate())

      await act(async () => {
        await result.current.mutate('template-001')
      })

      expect(mockDel).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockDel.mockRejectedValue(new Error('Delete failed'))
      const { result } = renderHook(() => useDeletePromptTemplate())

      await act(async () => {
        await expect(result.current.mutate('template-001')).rejects.toThrow('Delete failed')
      })

      expect(result.current.error).toBe('Delete failed')
      expect(result.current.loading).toBe(false)
    })
  })
})
