import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@/test/render'
import { ContextNode } from './ContextNode'

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
  workflowStore: { patchStepLocal: mockPatchStepLocal },
}))

const baseProps = {
  id: 'ctx-001',
  type: 'contextNode',
  data: {
    kind: 'context' as const,
    label: 'System Prompt',
    content: 'You are a helpful assistant.',
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

describe('ContextNode', () => {
  it('renders header with node name', () => {
    render(<ContextNode {...baseProps} />)
    expect(screen.getByText('System Prompt')).toBeInTheDocument()
  })

  it('renders Context badge', () => {
    render(<ContextNode {...baseProps} />)
    expect(screen.getByText('Context')).toBeInTheDocument()
  })

  it('renders content area with code editor', () => {
    render(<ContextNode {...baseProps} />)
    expect(screen.getByTestId('code-editor')).toBeInTheDocument()
  })

  it('renders source handle but no target handle (source-only node)', () => {
    render(<ContextNode {...baseProps} />)
    expect(screen.getByTestId('handle-source')).toBeInTheDocument()
    expect(screen.queryByTestId('handle-target')).not.toBeInTheDocument()
  })

  it('renders node resizer', () => {
    render(<ContextNode {...baseProps} />)
    expect(screen.getByTestId('node-resizer')).toBeInTheDocument()
  })

  it('calls useProtocolHighlight with correct arguments', () => {
    render(<ContextNode {...baseProps} />)
    expect(mockUseProtocolHighlight).toHaveBeenCalledWith('context', 'ctx-001', null)
  })

  it('passes protocolStepId to useProtocolHighlight when set', () => {
    const props = { ...baseProps, data: { ...baseProps.data, protocolStepId: 'step-proto' } }
    render(<ContextNode {...props} />)
    expect(mockUseProtocolHighlight).toHaveBeenCalledWith('context', 'ctx-001', 'step-proto')
  })

  it('calls patchStepLocal when content changes', () => {
    render(<ContextNode {...baseProps} />)
    screen.getByTestId('code-editor').click()
    expect(mockPatchStepLocal).toHaveBeenCalledWith('ctx-001', { prompt_template: 'updated' })
  })
})
