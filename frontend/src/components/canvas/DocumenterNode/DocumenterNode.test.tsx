import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { DocumenterNode } from '.'
import { mockWorkflowStep } from '@/test/fixtures'
import type { DocumentDef } from '@/types/workflow'

const {
  mockFetchDocumentDefs,
  mockPatchStepLocal,
  mockCreateDocumentDef,
  mockDeleteDocumentDef,
  _documentDefs,
  _hoveredStepId,
} = vi.hoisted(() => ({
  mockFetchDocumentDefs: vi.fn(),
  mockPatchStepLocal: vi.fn(),
  mockCreateDocumentDef: vi.fn(),
  mockDeleteDocumentDef: vi.fn(),
  _documentDefs: { value: [] as DocumentDef[] },
  _hoveredStepId: { value: null as string | null },
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') {
      const mockState = {
        documentDefsByStep: { 'step-doc-001': _documentDefs.value },
        hoveredStepId: _hoveredStepId.value,
      }
      return (selector as (s: unknown) => unknown)(mockState)
    }
    return undefined
  }),
  workflowStore: {
    store: 'workflow',
    selectStepDocumentDefs: (stepId: string) => (s: { documentDefsByStep: Record<string, DocumentDef[]> }) =>
      s.documentDefsByStep[stepId] ?? [],
    fetchDocumentDefs: mockFetchDocumentDefs,
    patchStepLocal: mockPatchStepLocal,
    createDocumentDef: mockCreateDocumentDef,
    deleteDocumentDef: mockDeleteDocumentDef,
  },
  canvasStore: {
    store: 'canvas',
  },
}))

vi.mock('@xyflow/react', () => ({
  Handle: ({ type, position, id }: { type: string; position: string; id?: string }) => (
    <div data-testid={id ? `handle-${type}-${id}` : `handle-${type}`} data-position={position} />
  ),
  Position: { Left: 'left', Right: 'right', Top: 'top', Bottom: 'bottom' },
  NodeResizer: () => <div data-testid="node-resizer" />,
}))

vi.mock('@/components/primitives/CodeEditor', () => ({
  CodeEditor: ({ value, placeholder }: { value: string; placeholder?: string }) => (
    <div data-testid="code-editor">{value || placeholder}</div>
  ),
}))

const mockDocumenterStep = {
  ...mockWorkflowStep,
  id: 'step-doc-001',
  name: 'Write Docs',
  execution_mode: 'documenter' as const,
}

const baseProps = {
  id: mockDocumenterStep.id,
  type: 'documenterNode',
  data: {
    kind: 'protocol' as const,
    label: 'Write Docs',
    documentNames: ['README', 'CHANGELOG'],
    upstreamStepNames: ['Parse Input'],
    promptValue: 'Generate documentation for the project.',
    modelId: 'claude-sonnet-4-20250514',
    agentName: 'DocBot',
  },
  selected: false,
  isConnectable: true,
  zIndex: 0,
  positionAbsoluteX: 0,
  positionAbsoluteY: 0,
  dragging: false,
  dragHandle: undefined,
  parentId: undefined,
  sourcePosition: undefined,
  targetPosition: undefined,
  width: 360,
  height: 420,
  measured: { width: 360, height: 420 },
  deletable: true,
  selectable: true,
  connectable: true,
  focusable: true,
}

const mockDocumentDefs: DocumentDef[] = [
  {
    id: 'def-001',
    step_id: 'step-doc-001',
    name: 'README',
    description: 'Project readme file',
    target_length: 5000,
    display_order: 0,
    created_at: '2025-01-01T00:00:00Z',
  },
  {
    id: 'def-002',
    step_id: 'step-doc-001',
    name: 'CHANGELOG',
    description: null,
    target_length: 2000,
    display_order: 1,
    created_at: '2025-01-01T00:00:00Z',
  },
]

beforeEach(() => {
  vi.clearAllMocks()
  _documentDefs.value = []
  _hoveredStepId.value = null
})

describe('DocumenterNode', () => {
  it('renders with provided data', () => {
    render(<DocumenterNode {...baseProps} />)
    expect(screen.getByText('Write Docs')).toBeInTheDocument()
    expect(screen.getByText('Protocol')).toBeInTheDocument()
  })

  it('shows documenter header with step name', () => {
    render(<DocumenterNode {...baseProps} />)
    expect(screen.getByText('Write Docs')).toBeInTheDocument()
  })

  it('shows document names summary in header', () => {
    render(<DocumenterNode {...baseProps} />)
    expect(screen.getByText('README \u00b7 CHANGELOG')).toBeInTheDocument()
  })

  it('shows "No documents" when documentNames is empty', () => {
    const props = {
      ...baseProps,
      data: { ...baseProps.data, documentNames: [] },
    }
    render(<DocumenterNode {...props} />)
    expect(screen.getByText('No documents')).toBeInTheDocument()
  })

  it('does not fetch document defs (parent canvas handles this)', () => {
    render(<DocumenterNode {...baseProps} />)
    expect(mockFetchDocumentDefs).not.toHaveBeenCalled()
  })

  describe('tab switching', () => {
    it('renders all four tab icons', () => {
      render(<DocumenterNode {...baseProps} />)
      expect(screen.getByTestId('tab-prompt')).toBeInTheDocument()
      expect(screen.getByTestId('tab-documents')).toBeInTheDocument()
      expect(screen.getByTestId('tab-inputs')).toBeInTheDocument()
      expect(screen.getByTestId('tab-settings')).toBeInTheDocument()
    })

    it('defaults to prompt tab with editor content', () => {
      render(<DocumenterNode {...baseProps} />)
      expect(screen.getByTestId('code-editor')).toBeInTheDocument()
      expect(screen.getByTestId('code-editor').textContent).toBe(
        'Generate documentation for the project.',
      )
    })

    it('switches to documents tab on click', () => {
      render(<DocumenterNode {...baseProps} />)
      fireEvent.click(screen.getByTestId('tab-documents'))
      expect(screen.getByText('Add Document')).toBeInTheDocument()
    })

    it('switches to inputs tab on click', () => {
      render(<DocumenterNode {...baseProps} />)
      fireEvent.click(screen.getByTestId('tab-inputs'))
      expect(screen.getByText('Upstream Inputs')).toBeInTheDocument()
      expect(screen.getByText('Parse Input')).toBeInTheDocument()
    })

    it('switches to settings tab on click', () => {
      render(<DocumenterNode {...baseProps} />)
      fireEvent.click(screen.getByTestId('tab-settings'))
      expect(screen.getByText('Name')).toBeInTheDocument()
      expect(screen.getByDisplayValue('Write Docs')).toBeInTheDocument()
    })
  })

  describe('documents tab', () => {
    it('shows "No documents configured" when no document defs exist', () => {
      _documentDefs.value = []
      render(<DocumenterNode {...baseProps} />)
      fireEvent.click(screen.getByTestId('tab-documents'))
      expect(screen.getByText('No documents configured')).toBeInTheDocument()
    })

    it('renders document definitions when present', () => {
      _documentDefs.value = mockDocumentDefs
      render(<DocumenterNode {...baseProps} />)
      fireEvent.click(screen.getByTestId('tab-documents'))
      expect(screen.getByText('README')).toBeInTheDocument()
      expect(screen.getByText('Project readme file')).toBeInTheDocument()
      expect(screen.getByText('5000 chars')).toBeInTheDocument()
      expect(screen.getByText('CHANGELOG')).toBeInTheDocument()
      expect(screen.getByText('2000 chars')).toBeInTheDocument()
    })
  })

  describe('handles', () => {
    it('renders target and source handles', () => {
      render(<DocumenterNode {...baseProps} />)
      expect(screen.getByTestId('handle-target')).toBeInTheDocument()
      expect(screen.getByTestId('handle-source')).toBeInTheDocument()
    })

    it('renders extra documents source handle at top', () => {
      render(<DocumenterNode {...baseProps} />)
      const topHandle = screen.getByTestId('handle-source-documents')
      expect(topHandle).toBeInTheDocument()
      expect(topHandle.dataset.position).toBe('top')
    })
  })
})
