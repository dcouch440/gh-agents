import { renderHook, act } from '@testing-library/react'
import { useCreateOutputSchema, useUpdateOutputSchema, useDeleteOutputSchema } from './useOutputSchemaMutations'
import { mockOutputSchema } from '@/test/fixtures'

const { mockPost, mockPut, mockDel } = vi.hoisted(() => ({
  mockPost: vi.fn(),
  mockPut: vi.fn(),
  mockDel: vi.fn(),
}))

const mockReload = vi.fn()

vi.mock('@/api', () => ({ api: { post: mockPost, put: mockPut, del: mockDel } }))
vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})
vi.mock('@/hooks/useOutputSchemaContext', () => ({
  useOutputSchemaContext: () => ({ reload: mockReload }),
}))

describe('useOutputSchemaMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useCreateOutputSchema', () => {
    it('creates a schema and calls reload', async () => {
      mockPost.mockResolvedValue(mockOutputSchema)
      const { result } = renderHook(() => useCreateOutputSchema())

      let schema: unknown
      await act(async () => {
        schema = await result.current.mutate({ name: 'Test Schema', json_schema: { type: 'object' } })
      })

      expect(schema).toEqual(mockOutputSchema)
      expect(mockPost).toHaveBeenCalledWith('/output-schemas', { name: 'Test Schema', json_schema: { type: 'object' } })
      expect(mockReload).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Create failed'))
      const { result } = renderHook(() => useCreateOutputSchema())

      await act(async () => {
        await expect(result.current.mutate({ name: 'Test', json_schema: {} })).rejects.toThrow('Create failed')
      })

      expect(result.current.error).toBe('Create failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useUpdateOutputSchema', () => {
    it('updates a schema and calls reload', async () => {
      mockPut.mockResolvedValue(mockOutputSchema)
      const { result } = renderHook(() => useUpdateOutputSchema())

      let schema: unknown
      await act(async () => {
        schema = await result.current.mutate('schema-001', { name: 'Updated' })
      })

      expect(schema).toEqual(mockOutputSchema)
      expect(mockPut).toHaveBeenCalledWith('/output-schemas/schema-001', { name: 'Updated' })
      expect(mockReload).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPut.mockRejectedValue(new Error('Update failed'))
      const { result } = renderHook(() => useUpdateOutputSchema())

      await act(async () => {
        await expect(result.current.mutate('schema-001', { name: 'Updated' })).rejects.toThrow('Update failed')
      })

      expect(result.current.error).toBe('Update failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useDeleteOutputSchema', () => {
    it('deletes a schema and calls reload', async () => {
      mockDel.mockResolvedValue(undefined)
      const { result } = renderHook(() => useDeleteOutputSchema())

      await act(async () => {
        await result.current.mutate('schema-001')
      })

      expect(mockDel).toHaveBeenCalledWith('/output-schemas/schema-001')
      expect(mockReload).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockDel.mockRejectedValue(new Error('Delete failed'))
      const { result } = renderHook(() => useDeleteOutputSchema())

      await act(async () => {
        await expect(result.current.mutate('schema-001')).rejects.toThrow('Delete failed')
      })

      expect(result.current.error).toBe('Delete failed')
      expect(result.current.loading).toBe(false)
    })
  })
})
