import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { CanvasContextMenu } from './CanvasContextMenu'

const { mockCreateStep } = vi.hoisted(() => ({
  mockCreateStep: vi.fn(),
}))

vi.mock('@/stores', () => ({
  workflowStore: {
    createStep: mockCreateStep,
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
})

const defaultPosition = { x: 200, y: 300, flowX: 150.5, flowY: 250.7 }

describe('CanvasContextMenu', () => {
  it('renders nothing when position is null', () => {
    const { container } = render(
      <CanvasContextMenu position={null} onClose={vi.fn()} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('renders menu with step type options', () => {
    render(
      <CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />,
    )
    expect(screen.getByText('Add Step')).toBeInTheDocument()
    expect(screen.getByText('LLM Step')).toBeInTheDocument()
    expect(screen.getByText('For-Each Step')).toBeInTheDocument()
    expect(screen.getByText('Router Step')).toBeInTheDocument()
    expect(screen.getByText('Human Review')).toBeInTheDocument()
    expect(screen.getByText('Tool Step')).toBeInTheDocument()
  })

  it('calls createStep with correct type and rounded position on click', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(
      <CanvasContextMenu position={defaultPosition} onClose={onClose} />,
    )

    await user.click(screen.getByText('LLM Step'))

    expect(mockCreateStep).toHaveBeenCalledWith({
      name: 'New llm step',
      step_type: 'llm',
      position_x: 151,
      position_y: 251,
    })
  })

  it('calls onClose after adding a step', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(
      <CanvasContextMenu position={defaultPosition} onClose={onClose} />,
    )

    await user.click(screen.getByText('For-Each Step'))
    expect(onClose).toHaveBeenCalledOnce()
  })

  it('creates for_each step type correctly', async () => {
    const user = userEvent.setup()
    render(
      <CanvasContextMenu position={defaultPosition} onClose={vi.fn()} />,
    )

    await user.click(screen.getByText('For-Each Step'))

    expect(mockCreateStep).toHaveBeenCalledWith(
      expect.objectContaining({ step_type: 'for_each' }),
    )
  })
})
