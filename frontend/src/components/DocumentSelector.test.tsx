import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { DocumentSelector } from './DocumentSelector'
import { mockDocument } from '@/test/fixtures'
import type { DocumentListItem } from '@/types/document'

const toListItem = (doc: typeof mockDocument): DocumentListItem => ({
  id: doc.id,
  title: doc.title,
  summary: doc.summary,
  ref_tag: doc.ref_tag,
  tags: doc.tags,
  doc_type: doc.doc_type,
  updated_at: doc.updated_at,
})

const doc1 = toListItem(mockDocument)
const doc2 = toListItem({
  ...mockDocument,
  id: 'doc-002',
  title: 'Second document',
  doc_type: 'spec',
  ref_tag: 'spec-ref',
})
const doc3 = toListItem({
  ...mockDocument,
  id: 'doc-003',
  title: 'Bare document',
  doc_type: null,
  ref_tag: null,
})

const {
  mockFetchAll,
  mockToggleExpand,
  mockGetDocumentContent,
  _documents,
  _loading,
  _expandedId,
} = vi.hoisted(() => ({
  mockFetchAll: vi.fn(),
  mockToggleExpand: vi.fn(),
  mockGetDocumentContent: vi.fn(() => 'mock content'),
  _documents: { value: [] as DocumentListItem[] },
  _loading: { value: false },
  _expandedId: { value: null as string | null },
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  documentStore: {
    store: 'document',
    selectAll: () => _documents.value,
    selectLoading: () => _loading.value,
    fetchAll: mockFetchAll,
  },
}))

vi.mock('./useDocumentExpand', () => ({
  useDocumentExpand: () => ({
    expandedId: _expandedId.value,
    loadingDocId: null,
    toggleExpand: mockToggleExpand,
    getDocumentContent: mockGetDocumentContent,
  }),
}))

const defaultProps = {
  selectedIds: [] as string[],
  onSelectionChange: vi.fn(),
  open: true,
  onClose: vi.fn(),
}

beforeEach(() => {
  vi.clearAllMocks()
  _documents.value = [doc1, doc2]
  _loading.value = false
  _expandedId.value = null
})

describe('DocumentSelector', () => {
  describe('visibility', () => {
    it('returns null when open is false', () => {
      const { container } = render(<DocumentSelector {...defaultProps} open={false} />)
      expect(container.firstChild).toBeNull()
    })
  })

  describe('loading state', () => {
    it('renders loading spinner when loading', () => {
      _loading.value = true
      render(<DocumentSelector {...defaultProps} />)
      expect(screen.getByRole('progressbar')).toBeInTheDocument()
    })
  })

  describe('empty state', () => {
    it('renders empty state when no documents', () => {
      _documents.value = []
      render(<DocumentSelector {...defaultProps} />)
      expect(screen.getByText('No documents available')).toBeInTheDocument()
    })
  })

  describe('document list', () => {
    it('renders document list with titles', () => {
      render(<DocumentSelector {...defaultProps} />)
      expect(screen.getByText('Test document')).toBeInTheDocument()
      expect(screen.getByText('Second document')).toBeInTheDocument()
    })

    it('shows doc_type chip and ref_tag when present', () => {
      render(<DocumentSelector {...defaultProps} />)
      expect(screen.getByText('note')).toBeInTheDocument()
      expect(screen.getByText('test-doc')).toBeInTheDocument()
      expect(screen.getByText('spec')).toBeInTheDocument()
      expect(screen.getByText('spec-ref')).toBeInTheDocument()
    })

    it('omits doc_type chip and ref_tag when null', () => {
      _documents.value = [doc3]
      render(<DocumentSelector {...defaultProps} />)
      expect(screen.getByText('Bare document')).toBeInTheDocument()
      expect(screen.queryByText('note')).not.toBeInTheDocument()
      expect(screen.queryByText('test-doc')).not.toBeInTheDocument()
    })
  })

  describe('selection toggling', () => {
    it('toggle selection adds and removes from set', async () => {
      const user = userEvent.setup()
      const onSelectionChange = vi.fn()
      render(
        <DocumentSelector
          {...defaultProps}
          onSelectionChange={onSelectionChange}
        />,
      )

      const checkboxes = screen.getAllByRole('checkbox')
      // Click first checkbox to select
      await user.click(checkboxes[0])

      // Save to verify the selected id is passed
      await user.click(screen.getByText('Save Selection'))
      expect(onSelectionChange).toHaveBeenCalledWith([doc1.id])
    })

    it('toggle selection removes already-selected id', async () => {
      const user = userEvent.setup()
      const onSelectionChange = vi.fn()
      render(
        <DocumentSelector
          {...defaultProps}
          selectedIds={[doc1.id]}
          onSelectionChange={onSelectionChange}
        />,
      )

      const checkboxes = screen.getAllByRole('checkbox')
      // Click first checkbox to deselect
      await user.click(checkboxes[0])

      await user.click(screen.getByText('Save Selection'))
      expect(onSelectionChange).toHaveBeenCalledWith([])
    })
  })

  describe('save action', () => {
    it('calls onSelectionChange with correct ids and onClose', async () => {
      const user = userEvent.setup()
      const onSelectionChange = vi.fn()
      const onClose = vi.fn()
      render(
        <DocumentSelector
          {...defaultProps}
          onSelectionChange={onSelectionChange}
          onClose={onClose}
        />,
      )

      // Select both documents
      const checkboxes = screen.getAllByRole('checkbox')
      await user.click(checkboxes[0])
      await user.click(checkboxes[1])

      await user.click(screen.getByText('Save Selection'))

      expect(onSelectionChange).toHaveBeenCalledOnce()
      const passedIds = onSelectionChange.mock.calls[0][0] as string[]
      expect(passedIds).toHaveLength(2)
      expect(passedIds).toContain(doc1.id)
      expect(passedIds).toContain(doc2.id)
      expect(onClose).toHaveBeenCalledOnce()
    })
  })

  describe('cancel action', () => {
    it('reverts selection and calls onClose', async () => {
      const user = userEvent.setup()
      const onSelectionChange = vi.fn()
      const onClose = vi.fn()
      render(
        <DocumentSelector
          {...defaultProps}
          selectedIds={[]}
          onSelectionChange={onSelectionChange}
          onClose={onClose}
        />,
      )

      // Select a document
      const checkboxes = screen.getAllByRole('checkbox')
      await user.click(checkboxes[0])

      // Cancel instead of save
      await user.click(screen.getByText('Cancel'))

      expect(onClose).toHaveBeenCalledOnce()
      expect(onSelectionChange).not.toHaveBeenCalled()
    })
  })
})
