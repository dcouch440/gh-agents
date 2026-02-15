import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@/test/render'
import userEvent from '@testing-library/user-event'
import { CanvasContextMenu } from './CanvasContextMenu'

const { mockCreateStep, mockDeleteStep, mockLinkStepProtocol, mockEnterShareMode } = vi.hoisted(() => ({
  mockCreateStep: vi.fn(),
  mockDeleteStep: vi.fn(),
  mockLinkStepProtocol: vi.fn(),
  mockEnterShareMode: vi.fn(),
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

const mockStep = {
  id: 'step-123',
  name: 'Test Step',
  description: 'A test step',
  execution_mode: 'documenter',
  prompt_template: '',
}

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  workflowStore: {
    store: {
      getState: () => ({
        steps: { byId: new Map([['step-123', mockStep]]), ids: ['step-123'] },
        documentDefsByStep: {},
        rosterByStep: {},
        roomMembersByStep: {},
      }),
    },
    createStep: mockCreateStep,
    deleteStep: mockDeleteStep,
    selectSteps: () => [mockStep],
  },
  protocolStore: {
    store: 'protocol',
    selectTypes: () => [],
    selectAll: () => [mockDocumenterProtocol],
  },
  canvasStore: {
    store: { getState: () => ({ stepProtocols: {} }) },
    linkStepProtocol: mockLinkStepProtocol,
  },
  shareStore: {
    store: { getState: () => ({ active: false }) },
    enterShareMode: mockEnterShareMode,
  },
}))

vi.mock('./buildShareableFields', () => ({
  buildShareableFields: vi.fn(() => []),
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

  it('renders Archetypes section header', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.getByText('Archetypes')).toBeInTheDocument()
  })

  it('renders Utilities section header', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.getByText('Utilities')).toBeInTheDocument()
  })

  it('renders three archetype options', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.getByTestId('ctx-add-documenter')).toBeInTheDocument()
    expect(screen.getByText('Documenter')).toBeInTheDocument()
    expect(screen.getByTestId('ctx-add-task_force')).toBeInTheDocument()
    expect(screen.getByText('Task Force')).toBeInTheDocument()
    expect(screen.getByTestId('ctx-add-room')).toBeInTheDocument()
    expect(screen.getByText('Room')).toBeInTheDocument()
  })

  it('renders Context under Utilities', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.getByTestId('ctx-add-context')).toBeInTheDocument()
    expect(screen.getByText('Context')).toBeInTheDocument()
  })

  it('renders Input under Utilities', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.getByTestId('ctx-add-input')).toBeInTheDocument()
    expect(screen.getByText('Input')).toBeInTheDocument()
  })

  it('creates input step with rounded position', async () => {
    const user = userEvent.setup()
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)

    await user.click(screen.getByText('Input'))

    expect(mockCreateStep).toHaveBeenCalledWith({
      name: 'Input',
      execution_mode: 'input',
      position_x: 151,
      position_y: 251,
    })
  })

  it('does not create input step when one already exists', async () => {
    // Override selectSteps to return an input step
    const { workflowStore: ws } = await import('@/stores')
    const inputStep = { ...mockStep, execution_mode: 'input' }
    vi.spyOn(ws, 'selectSteps' as never).mockReturnValue([inputStep])

    const user = userEvent.setup()
    const onClose = vi.fn()
    render(<CanvasContextMenu position={defaultPosition} onClose={onClose} />)

    await user.click(screen.getByText('Input'))

    expect(mockCreateStep).not.toHaveBeenCalled()
    expect(onClose).toHaveBeenCalledOnce()
  })

  it('does not render old step types', () => {
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)
    expect(screen.queryByText('LLM Step')).not.toBeInTheDocument()
    expect(screen.queryByText('For-Each Step')).not.toBeInTheDocument()
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

  it('creates room step on Room click', async () => {
    const user = userEvent.setup()
    render(<CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />)

    await user.click(screen.getByText('Room'))

    expect(mockCreateStep).toHaveBeenCalledWith({
      name: 'New Room',
      execution_mode: 'room',
      prompt_template: '',
      position_x: 151,
      position_y: 251,
    })
  })

  describe('node context menu', () => {
    const nodePosition = { ...defaultPosition, nodeId: 'step-123' }

    it('renders Share and Delete Step when position has nodeId', () => {
      render(<CanvasContextMenu position={nodePosition} onClose={vi.fn()} />)
      expect(screen.getByText('Share')).toBeInTheDocument()
      expect(screen.getByText('Delete Step')).toBeInTheDocument()
    })

    it('does not render Archetypes or Utilities when nodeId is present', () => {
      render(<CanvasContextMenu position={nodePosition} onClose={vi.fn()} />)
      expect(screen.queryByText('Archetypes')).not.toBeInTheDocument()
      expect(screen.queryByText('Utilities')).not.toBeInTheDocument()
    })

    it('calls enterShareMode on Share click', async () => {
      const onClose = vi.fn()
      const user = userEvent.setup()
      render(<CanvasContextMenu position={nodePosition} onClose={onClose} />)

      await user.click(screen.getByText('Share'))

      expect(mockEnterShareMode).toHaveBeenCalledWith('step-123', expect.any(Array))
      expect(onClose).toHaveBeenCalledOnce()
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

  describe('document artifact context menu', () => {
    const docPosition = { ...defaultPosition, nodeId: 'doc-artifact-def-1' }

    it('renders Share and Delete Step for doc-artifact nodes', () => {
      render(<CanvasContextMenu position={docPosition} onClose={vi.fn()} />)
      expect(screen.getByText('Share')).toBeInTheDocument()
      expect(screen.getByText('Delete Step')).toBeInTheDocument()
    })

    it('calls enterShareMode with doc-artifact ID on Share click', async () => {
      const onClose = vi.fn()
      const user = userEvent.setup()

      // Override store mock to include document defs for this test
      const { workflowStore: ws } = await import('@/stores')
      const original = ws.store.getState
      vi.spyOn(ws.store, 'getState').mockReturnValue({
        ...original(),
        documentDefsByStep: {
          'step-123': [{ id: 'def-1', step_id: 'step-123', name: 'README', description: 'Project readme', target_length: 500, display_order: 0, created_at: '2025-01-01' }],
        },
      })

      render(<CanvasContextMenu position={docPosition} onClose={onClose} />)
      await user.click(screen.getByText('Share'))

      expect(mockEnterShareMode).toHaveBeenCalledWith('doc-artifact-def-1', expect.any(Array))
      expect(onClose).toHaveBeenCalledOnce()
    })
  })
})
