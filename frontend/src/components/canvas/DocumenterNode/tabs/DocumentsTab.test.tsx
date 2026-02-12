import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { DocumentsTab } from './DocumentsTab'
import type { DocumentDef, CreateDocumentDefRequest } from '@/types/workflow'

const mockDocumentDefs: DocumentDef[] = [
  {
    id: 'def-001',
    step_id: 'step-1',
    name: 'README',
    description: 'Project readme',
    target_length: 5000,
    display_order: 0,
    created_at: '2025-01-01T00:00:00Z',
  },
  {
    id: 'def-002',
    step_id: 'step-1',
    name: 'CHANGELOG',
    description: null,
    target_length: 2000,
    display_order: 1,
    created_at: '2025-01-01T00:00:00Z',
  },
]

describe('DocumentsTab', () => {
  const onAdd = vi.fn()
  const onSubmitNew = vi.fn()
  const onCancelAdd = vi.fn()
  const onRemove = vi.fn()

  const baseProps = {
    documents: [] as DocumentDef[],
    adding: false,
    onAdd,
    onSubmitNew: onSubmitNew as (body: CreateDocumentDefRequest) => void,
    onCancelAdd,
    onRemove,
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders empty state when no documents exist', () => {
    render(<DocumentsTab {...baseProps} />)
    expect(screen.getByText('No documents configured')).toBeInTheDocument()
  })

  it('renders document list with names and target lengths', () => {
    render(<DocumentsTab {...baseProps} documents={mockDocumentDefs} />)
    expect(screen.getByText('README')).toBeInTheDocument()
    expect(screen.getByText('Project readme')).toBeInTheDocument()
    expect(screen.getByText('5000 chars')).toBeInTheDocument()
    expect(screen.getByText('CHANGELOG')).toBeInTheDocument()
    expect(screen.getByText('2000 chars')).toBeInTheDocument()
  })

  it('shows Add Document button when not adding', () => {
    render(<DocumentsTab {...baseProps} />)
    expect(screen.getByText('Add Document')).toBeInTheDocument()
  })

  it('calls onAdd when Add Document is clicked', () => {
    render(<DocumentsTab {...baseProps} />)
    fireEvent.click(screen.getByText('Add Document'))
    expect(onAdd).toHaveBeenCalled()
  })

  it('shows InlineAddForm when adding is true', () => {
    render(<DocumentsTab {...baseProps} adding={true} />)
    expect(screen.getByTestId('inline-add-name')).toBeInTheDocument()
    expect(screen.queryByText('Add Document')).not.toBeInTheDocument()
  })

  it('calls onRemove with the correct document id', () => {
    render(<DocumentsTab {...baseProps} documents={mockDocumentDefs} />)
    const removeButtons = screen.getAllByText('\u00d7')
    fireEvent.click(removeButtons[0]!)
    expect(onRemove).toHaveBeenCalledWith('def-001')
  })

  it('does not render description for documents without one', () => {
    const singleDoc: DocumentDef[] = [mockDocumentDefs[1]!]
    render(<DocumentsTab {...baseProps} documents={singleDoc} />)
    expect(screen.getByText('CHANGELOG')).toBeInTheDocument()
    expect(screen.queryByText('Project readme')).not.toBeInTheDocument()
  })
})
