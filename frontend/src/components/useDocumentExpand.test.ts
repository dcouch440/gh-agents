import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act, waitFor } from '@testing-library/react'
import { useDocumentExpand } from './useDocumentExpand'
import type { DocumentListItem } from '@/types/document'

const { mockDocumentGet } = vi.hoisted(() => ({
  mockDocumentGet: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    documents: {
      get: mockDocumentGet,
    },
  },
}))

const documents: DocumentListItem[] = [
  { id: 'doc-001', title: 'First Doc', summary: 'Summary of first doc', doc_type: 'note', ref_tag: 'first', tags: [], created_at: '2025-01-01T00:00:00Z', updated_at: '2025-01-01T00:00:00Z' },
  { id: 'doc-002', title: 'Second Doc', summary: 'Summary of second doc', doc_type: 'note', ref_tag: 'second', tags: [], created_at: '2025-01-01T00:00:00Z', updated_at: '2025-01-01T00:00:00Z' },
]

const fullDocument = {
  id: 'doc-001',
  user_id: 'user-001',
  session_id: null,
  title: 'First Doc',
  content: 'Full content of first document',
  summary: 'Summary of first doc',
  doc_type: 'note',
  ref_tag: 'first',
  tags: [],
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
}

const mockStopPropagation = vi.fn()
const mockEvent = { stopPropagation: mockStopPropagation } as unknown as React.MouseEvent

beforeEach(() => {
  vi.clearAllMocks()
})

describe('useDocumentExpand', () => {
  describe('initial state', () => {
    it('starts with expandedId as null', () => {
      const { result } = renderHook(() => useDocumentExpand(documents))
      expect(result.current.expandedId).toBeNull()
    })

    it('starts with loadingDocId as null', () => {
      const { result } = renderHook(() => useDocumentExpand(documents))
      expect(result.current.loadingDocId).toBeNull()
    })
  })

  describe('toggleExpand', () => {
    it('sets expandedId when toggling a document', () => {
      mockDocumentGet.mockReturnValue(new Promise(() => {}))
      const { result } = renderHook(() => useDocumentExpand(documents))

      act(() => {
        result.current.toggleExpand('doc-001', mockEvent)
      })

      expect(result.current.expandedId).toBe('doc-001')
      expect(mockStopPropagation).toHaveBeenCalled()
    })

    it('collapses when toggling the same document', () => {
      mockDocumentGet.mockReturnValue(new Promise(() => {}))
      const { result } = renderHook(() => useDocumentExpand(documents))

      act(() => {
        result.current.toggleExpand('doc-001', mockEvent)
      })
      act(() => {
        result.current.toggleExpand('doc-001', mockEvent)
      })

      expect(result.current.expandedId).toBeNull()
    })

    it('fetches document content on first expand', () => {
      mockDocumentGet.mockReturnValue(new Promise(() => {}))
      const { result } = renderHook(() => useDocumentExpand(documents))

      act(() => {
        result.current.toggleExpand('doc-001', mockEvent)
      })

      expect(mockDocumentGet).toHaveBeenCalledWith('doc-001')
    })

    it('does not re-fetch when expanding an already loaded document', async () => {
      mockDocumentGet.mockResolvedValue(fullDocument)
      const { result } = renderHook(() => useDocumentExpand(documents))

      // Expand and load
      act(() => {
        result.current.toggleExpand('doc-001', mockEvent)
      })
      await waitFor(() => {
        expect(result.current.loadingDocId).toBeNull()
      })

      // Collapse
      act(() => {
        result.current.toggleExpand('doc-001', mockEvent)
      })

      // Re-expand — should not fetch again
      mockDocumentGet.mockClear()
      act(() => {
        result.current.toggleExpand('doc-001', mockEvent)
      })

      expect(mockDocumentGet).not.toHaveBeenCalled()
    })
  })

  describe('getDocumentContent', () => {
    it('returns loading message while fetching', () => {
      mockDocumentGet.mockReturnValue(new Promise(() => {}))
      const { result } = renderHook(() => useDocumentExpand(documents))

      act(() => {
        result.current.toggleExpand('doc-001', mockEvent)
      })

      expect(result.current.getDocumentContent('doc-001')).toBe('Loading document content...')
    })

    it('returns full content after fetch completes', async () => {
      mockDocumentGet.mockResolvedValue(fullDocument)
      const { result } = renderHook(() => useDocumentExpand(documents))

      act(() => {
        result.current.toggleExpand('doc-001', mockEvent)
      })

      await waitFor(() => {
        expect(result.current.getDocumentContent('doc-001')).toBe('Full content of first document')
      })
    })

    it('returns summary as fallback for unloaded documents', () => {
      const { result } = renderHook(() => useDocumentExpand(documents))
      expect(result.current.getDocumentContent('doc-001')).toBe('Summary of first doc')
    })

    it('returns fallback text for unknown document id', () => {
      const { result } = renderHook(() => useDocumentExpand(documents))
      expect(result.current.getDocumentContent('nonexistent')).toBe('No content available')
    })
  })
})
