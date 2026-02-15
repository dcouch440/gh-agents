import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@/test/render'
import { InputNode } from './InputNode'

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
  CodeEditor: ({ value, onChange }: { value: string; onChange?: (v: string) => void }) => (
    <pre data-testid="code-editor" onClick={() => onChange?.('updated')}>{value}</pre>
  ),
}))

const { mockUseProtocolHighlight } = vi.hoisted(() => ({
  mockUseProtocolHighlight: vi.fn(() => 'none'),
}))

vi.mock('../useProtocolHighlight', () => ({
  useProtocolHighlight: mockUseProtocolHighlight,
}))

const { mockPatchStepLocal } = vi.hoisted(() => ({
  mockPatchStepLocal: vi.fn(),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn(() => false),
  workflowStore: {
    patchStepLocal: mockPatchStepLocal,
    selectActiveWorkflowId: vi.fn(() => null),
    selectStepById: vi.fn(() => () => null),
    store: { getState: vi.fn(), subscribe: vi.fn() },
  },
  shareStore: {
    store: 'share',
  },
  canvasStore: {
    store: 'canvas',
  },
}))

const baseProps = {
  id: 'input-001',
  type: 'inputNode',
  data: {
    kind: 'input' as const,
    label: 'Workflow Input',
    content: 'Project contains 2 categories.',
    protocolColor: null,
    protocolStepId: null,
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

describe('InputNode', () => {
  it('renders header with node name', () => {
    render(<InputNode {...baseProps} />)
    expect(screen.getByText('Workflow Input')).toBeInTheDocument()
  })

  it('renders Input badge', () => {
    render(<InputNode {...baseProps} />)
    expect(screen.getByText('Input')).toBeInTheDocument()
  })

  it('renders subtitle text', () => {
    render(<InputNode {...baseProps} />)
    expect(screen.getByText('Editable input for each run')).toBeInTheDocument()
  })

  it('renders content area with code editor', () => {
    render(<InputNode {...baseProps} />)
    expect(screen.getByTestId('code-editor')).toBeInTheDocument()
  })

  it('renders source handle but no target handle (source-only node)', () => {
    render(<InputNode {...baseProps} />)
    expect(screen.getByTestId('handle-source')).toBeInTheDocument()
    expect(screen.queryByTestId('handle-target')).not.toBeInTheDocument()
  })

  it('renders node resizer', () => {
    render(<InputNode {...baseProps} />)
    expect(screen.getByTestId('node-resizer')).toBeInTheDocument()
  })

  it('calls useProtocolHighlight with INPUT kind', () => {
    render(<InputNode {...baseProps} />)
    expect(mockUseProtocolHighlight).toHaveBeenCalledWith('input', 'input-001', null)
  })

  it('passes protocolStepId to useProtocolHighlight when set', () => {
    const props = { ...baseProps, data: { ...baseProps.data, protocolStepId: 'step-proto' } }
    render(<InputNode {...props} />)
    expect(mockUseProtocolHighlight).toHaveBeenCalledWith('input', 'input-001', 'step-proto')
  })

  it('calls patchStepLocal when content changes', () => {
    render(<InputNode {...baseProps} />)
    screen.getByTestId('code-editor').click()
    expect(mockPatchStepLocal).toHaveBeenCalledWith('input-001', { prompt_template: 'updated' })
  })
})
