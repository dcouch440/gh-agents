import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { CanvasContextMenu } from './CanvasContextMenu'

const { mockCreateStep, mockDeleteStep, mockLinkStepProtocol } = vi.hoisted(() => ({
  mockCreateStep: vi.fn(),
  mockDeleteStep: vi.fn(),
  mockLinkStepProtocol: vi.fn(),
}))

const mockDocumenterProtocol = {
  id: 'proto-1',
  name: 'Documenter',
  protocol_type: 'documenter',
  agent: { id: 'agent-1' },
  output_schema: { id: 'schema-1' },
  prompt_template: { id: 'template-1', content: 'doc prompt' },
  ports: [{ port_name: 'output' }],
}

const mockProtocolTypes = [{ name: 'documenter' }]

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  workflowStore: {
    store: 'workflow',
    createStep: mockCreateStep,
    deleteStep: mockDeleteStep,
  },
  protocolStore: {
    store: 'protocol',
    selectTypes: () => mockProtocolTypes,
    selectAll: () => [mockDocumenterProtocol],
  },
  canvasStore: {
    linkStepProtocol: mockLinkStepProtocol,
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
})

const defaultPosition = { x: 200, y: 300, flowX: 150.5, flowY: 250.7 }

describe('CanvasContextMenu', () => {
  it('renders nothing when position is null', () => {
    const { container } = render(<CanvasContextMenu position={null} onClose={vi.fn()} />)
    expect(container.firstChild).toBeNull()
  })

  it('renders Protocols section header', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.getByText('Protocols')).toBeInTheDocument()
  })

  it('renders Utilities section header', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.getByText('Utilities')).toBeInTheDocument()
  })

  it('renders Documenter under Protocols', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.getByTestId('ctx-add-documenter')).toBeInTheDocument()
    expect(screen.getByText('Documenter')).toBeInTheDocument()
  })

  it('renders Context under Utilities', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.getByTestId('ctx-add-context')).toBeInTheDocument()
    expect(screen.getByText('Context')).toBeInTheDocument()
  })

  it('does not render old step types', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.queryByText('LLM Step')).not.toBeInTheDocument()
    expect(screen.queryByText('For-Each Step')).not.toBeInTheDocument()
    expect(screen.queryByText('Room Step')).not.toBeInTheDocument()
    expect(screen.queryByText('Add Step')).not.toBeInTheDocument()
  })

  it('calls onClose after adding context', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<CanvasContextMenu position={defaultPosition} onClose={onClose} />)

    await user.click(screen.getByText('Context'))
    expect(onClose).toHaveBeenCalledOnce()
  })

  it('creates context step with rounded position', async () => {
    const user = userEvent.setup()
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)

    await user.click(screen.getByText('Context'))

    expect(mockCreateStep).toHaveBeenCalledWith({
      name: 'Context',
      execution_mode: 'context',
      position_x: 151,
      position_y: 251,
    })
  })

  describe('node context menu', () => {
    const nodePosition = { ...defaultPosition, nodeId: 'step-123' }

    it('renders Delete Step when position has nodeId', () => {
      render(<CanvasContextMenu position={nodePosition} onClose={vi.fn()} />)
      expect(screen.getByText('Delete Step')).toBeInTheDocument()
    })

    it('does not render Protocols or Utilities when nodeId is present', () => {
      render(<CanvasContextMenu position={nodePosition} onClose={vi.fn()} />)
      expect(screen.queryByText('Protocols')).not.toBeInTheDocument()
      expect(screen.queryByText('Utilities')).not.toBeInTheDocument()
    })

    it('calls deleteStep with correct node ID on click', async () => {
      const user = userEvent.setup()
      render(<CanvasContextMenu position={nodePosition} onClose={vi.fn()} />)

      await user.click(screen.getByText('Delete Step'))

      expect(mockDeleteStep).toHaveBeenCalledWith('step-123')
    })

    it('calls onClose after delete', async () => {
      const onClose = vi.fn()
      const user = userEvent.setup()
      render(<CanvasContextMenu position={nodePosition} onClose={onClose} />)

      await user.click(screen.getByText('Delete Step'))
      expect(onClose).toHaveBeenCalledOnce()
    })
  })
})
