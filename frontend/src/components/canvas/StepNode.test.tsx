import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { StepNode } from './StepNode'

vi.mock('@xyflow/react', () => ({
  Handle: ({ type, position }: { type: string; position: string }) => (
    <div data-testid={`handle-${type}`} data-position={position} />
  ),
  Position: { Left: 'left', Right: 'right' },
}))

vi.mock('@/constants', () => ({
  DESIGN: {
    BG_HEADER: '#0d1017',
  },
}))

const baseProps = {
  id: 'step-001',
  type: 'stepNode',
  data: {
    label: 'My Step',
    stepType: 'llm',
    description: 'A test step',
    agentId: null,
    promptTemplateId: null,
    outputSchemaId: null,
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
  width: 260,
  height: 100,
  measured: { width: 260, height: 100 },
  deletable: true,
  selectable: true,
  connectable: true,
  focusable: true,
}

describe('StepNode', () => {
  it('renders node label', () => {
    render(<StepNode {...baseProps} />)
    expect(screen.getByText('My Step')).toBeInTheDocument()
  })

  it('renders step type badge', () => {
    render(<StepNode {...baseProps} />)
    expect(screen.getByText('llm')).toBeInTheDocument()
  })

  it('renders description text', () => {
    render(<StepNode {...baseProps} />)
    expect(screen.getByText('A test step')).toBeInTheDocument()
  })

  it('renders "No description" when description is null', () => {
    const props = {
      ...baseProps,
      data: { ...baseProps.data, description: null },
    }
    render(<StepNode {...props} />)
    expect(screen.getByText('No description')).toBeInTheDocument()
  })

  it('renders input and output handles', () => {
    render(<StepNode {...baseProps} />)
    expect(screen.getByTestId('handle-target')).toBeInTheDocument()
    expect(screen.getByTestId('handle-source')).toBeInTheDocument()
  })

  it('renders with for_each step type', () => {
    const props = {
      ...baseProps,
      data: { ...baseProps.data, stepType: 'for_each', label: 'Loop Step' },
    }
    render(<StepNode {...props} />)
    expect(screen.getByText('Loop Step')).toBeInTheDocument()
    expect(screen.getByText('for_each')).toBeInTheDocument()
  })
})
