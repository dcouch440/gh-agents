import { renderHook, act } from '@testing-library/react'
import { useCreateDocument, useUpdateDocument, useDeleteDocument, useSearchDocuments } from './useDocumentMutations'
import { mockDocument } from '@/test/fixtures'
import type { DocumentSearchResult } from '@/types'

const { mockGet, mockPost, mockPatch, mockDel } = vi.hoisted(() => ({
  mockGet: vi.fn(),
  mockPost: vi.fn(),
  mockPatch: vi.fn(),
  mockDel: vi.fn(),
}))

vi.mock('@/api', () => ({ api: { get: mockGet, post: mockPost, patch: mockPatch, del: mockDel } }))
vi.mock('@/constants', async () => {
  const actual = await vi.importActual<Record<string, unknown>>('@/constants')
  return { ...actual, USE_MOCK_DATA: false }
})

describe('useDocumentMutations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('useCreateDocument', () => {
    it('creates a document and returns it', async () => {
      mockPost.mockResolvedValue(mockDocument)
      const { result } = renderHook(() => useCreateDocument())

      let doc: unknown
      await act(async () => {
        doc = await result.current.mutate({ title: 'Test document', content: 'Document content here', doc_type: 'note', ref_tag: 'test-doc', tags: ['test'] })
      })

      expect(doc).toEqual(mockDocument)
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPost.mockRejectedValue(new Error('Create failed'))
      const { result } = renderHook(() => useCreateDocument())

      await act(async () => {
        await expect(result.current.mutate({ title: 'Test', content: 'Content', doc_type: 'note', ref_tag: 'test', tags: [] })).rejects.toThrow('Create failed')
      })

      expect(result.current.error).toBe('Create failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useUpdateDocument', () => {
    it('updates a document and returns it', async () => {
      mockPatch.mockResolvedValue(mockDocument)
      const { result } = renderHook(() => useUpdateDocument())

      let doc: unknown
      await act(async () => {
        doc = await result.current.mutate('doc-001', { title: 'Updated' })
      })

      expect(doc).toEqual(mockDocument)
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockPatch.mockRejectedValue(new Error('Update failed'))
      const { result } = renderHook(() => useUpdateDocument())

      await act(async () => {
        await expect(result.current.mutate('doc-001', { title: 'Updated' })).rejects.toThrow('Update failed')
      })

      expect(result.current.error).toBe('Update failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useDeleteDocument', () => {
    it('deletes a document', async () => {
      mockDel.mockResolvedValue(undefined)
      const { result } = renderHook(() => useDeleteDocument())

      await act(async () => {
        await result.current.mutate('doc-001')
      })

      expect(mockDel).toHaveBeenCalledOnce()
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockDel.mockRejectedValue(new Error('Delete failed'))
      const { result } = renderHook(() => useDeleteDocument())

      await act(async () => {
        await expect(result.current.mutate('doc-001')).rejects.toThrow('Delete failed')
      })

      expect(result.current.error).toBe('Delete failed')
      expect(result.current.loading).toBe(false)
    })
  })

  describe('useSearchDocuments', () => {
    const mockSearchResult: DocumentSearchResult = {
      id: 'doc-001',
      title: 'Test document',
      summary: 'A test doc',
      ref_tag: 'test-doc',
      tags: ['test'],
      doc_type: 'note',
      score: 0.95,
    }

    it('searches documents and returns results', async () => {
      mockGet.mockResolvedValue([mockSearchResult])
      const { result } = renderHook(() => useSearchDocuments())

      await act(async () => {
        await result.current.search('test query')
      })

      expect(result.current.results).toEqual([mockSearchResult])
      expect(result.current.loading).toBe(false)
      expect(result.current.error).toBeNull()
    })

    it('sets error and throws on failure', async () => {
      mockGet.mockRejectedValue(new Error('Search failed'))
      const { result } = renderHook(() => useSearchDocuments())

      await act(async () => {
        await expect(result.current.search('test query')).rejects.toThrow('Search failed')
      })

      expect(result.current.error).toBe('Search failed')
      expect(result.current.loading).toBe(false)
    })
  })
})
