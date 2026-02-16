import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@/test/render'
import { DocumentNode } from './DocumentNode'

vi.mock('../useCanvasLOD', () => ({ useCanvasLOD: () => 'full' }))

vi.mock('@xyflow/react', () => ({
  Handle: ({ type, position }: { type: string; position: string }) => (
    <div data-testid={`handle-${type}`} data-position={position} />
  ),
  Position: { Top: 'top', Bottom: 'bottom', Left: 'left', Right: 'right' },
  NodeResizer: () => <div data-testid="node-resizer" />,
}))

vi.mock('@/components/primitives/MarkdownPreview', () => ({
  MarkdownPreview: ({ content }: { content: string }) => <div data-testid="markdown-preview">{content}</div>,
}))

vi.mock('@/components/primitives/CodeEditor', () => ({
  CodeEditor: ({ value }: { value: string }) => <pre data-testid="code-editor">{value}</pre>,
}))

const { mockUseProtocolHighlight } = vi.hoisted(() => ({
  mockUseProtocolHighlight: vi.fn(() => 'none'),
}))

vi.mock('../useProtocolHighlight', () => ({
  useProtocolHighlight: mockUseProtocolHighlight,
}))

const baseProps = {
  id: 'doc-artifact-001',
  type: 'documentNode',
  data: {
    kind: 'document' as const,
    label: 'API Specification',
    parentStepName: 'Doc Writer',
    content: '# API\n\nEndpoint details.',
    protocolStepId: 'step-001',
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
  width: 420,
  height: 360,
  measured: { width: 420, height: 360 },
  deletable: true,
  selectable: true,
  connectable: true,
  focusable: true,
}

describe('DocumentNode', () => {
  it('renders document name in header', () => {
    render(<DocumentNode {...baseProps} />)
    expect(screen.getByText('API Specification')).toBeInTheDocument()
  })

  it('renders parent step name in header', () => {
    render(<DocumentNode {...baseProps} />)
    expect(screen.getByText('Doc Writer')).toBeInTheDocument()
  })

  it('renders Document badge', () => {
    render(<DocumentNode {...baseProps} />)
    expect(screen.getByText('Document')).toBeInTheDocument()
  })

  it('renders markdown preview with content by default', () => {
    render(<DocumentNode {...baseProps} />)
    expect(screen.getByTestId('markdown-preview')).toBeInTheDocument()
  })

  it('renders target handle for incoming edges', () => {
    render(<DocumentNode {...baseProps} />)
    expect(screen.getByTestId('handle-target')).toBeInTheDocument()
  })

  it('renders empty state when content is empty', () => {
    const props = { ...baseProps, data: { ...baseProps.data, content: '' } }
    render(<DocumentNode {...props} />)
    expect(screen.getByText('Document will be generated when workflow runs.')).toBeInTheDocument()
  })

  it('calls useProtocolHighlight with correct arguments', () => {
    render(<DocumentNode {...baseProps} />)
    expect(mockUseProtocolHighlight).toHaveBeenCalledWith('document', 'doc-artifact-001', 'step-001')
  })

  it('handles null protocolStepId', () => {
    const props = { ...baseProps, data: { ...baseProps.data, protocolStepId: null } }
    render(<DocumentNode {...props} />)
    expect(mockUseProtocolHighlight).toHaveBeenCalledWith('document', 'doc-artifact-001', null)
    expect(screen.getByText('API Specification')).toBeInTheDocument()
  })
})
