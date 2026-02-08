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
    stepType: 'single',
    agentId: null,
    promptTemplateId: null,
    outputSchemaId: null,
    agentName: null,
    modelId: null,
    outputSchemaName: null,
    upstreamStepNames: [],
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
    expect(screen.getByText('single')).toBeInTheDocument()
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

  describe('subtitle', () => {
    it('renders agent name and model when both present', () => {
      const props = {
        ...baseProps,
        data: { ...baseProps.data, agentName: 'TestBot', modelId: 'claude-sonnet-4' },
      }
      render(<StepNode {...props} />)
      expect(screen.getByText('TestBot \u00b7 claude-sonnet-4')).toBeInTheDocument()
    })

    it('renders only agent name when modelId is null', () => {
      const props = {
        ...baseProps,
        data: { ...baseProps.data, agentName: 'TestBot', modelId: null },
      }
      render(<StepNode {...props} />)
      expect(screen.getByText('TestBot')).toBeInTheDocument()
    })

    it('does not render subtitle when agentName is null', () => {
      render(<StepNode {...baseProps} />)
      expect(screen.queryByText(/TestBot/)).not.toBeInTheDocument()
    })
  })

  describe('body sections', () => {
    it('renders upstream step pills in Inputs section', () => {
      const props = {
        ...baseProps,
        data: { ...baseProps.data, upstreamStepNames: ['Parse Input', 'Fetch Data'] },
      }
      render(<StepNode {...props} />)
      expect(screen.getByText('Inputs')).toBeInTheDocument()
      expect(screen.getByText('Parse Input')).toBeInTheDocument()
      expect(screen.getByText('Fetch Data')).toBeInTheDocument()
    })

    it('renders output schema name in Output section', () => {
      const props = {
        ...baseProps,
        data: { ...baseProps.data, outputSchemaName: 'ReviewSchema' },
      }
      render(<StepNode {...props} />)
      expect(screen.getByText('Output')).toBeInTheDocument()
      expect(screen.getByText('ReviewSchema')).toBeInTheDocument()
    })

    it('does not render body when no inputs and no output schema', () => {
      render(<StepNode {...baseProps} />)
      expect(screen.queryByText('Inputs')).not.toBeInTheDocument()
      expect(screen.queryByText('Output')).not.toBeInTheDocument()
    })

    it('renders both Inputs and Output when both present', () => {
      const props = {
        ...baseProps,
        data: {
          ...baseProps.data,
          upstreamStepNames: ['Step A'],
          outputSchemaName: 'MySchema',
        },
      }
      render(<StepNode {...props} />)
      expect(screen.getByText('Inputs')).toBeInTheDocument()
      expect(screen.getByText('Step A')).toBeInTheDocument()
      expect(screen.getByText('Output')).toBeInTheDocument()
      expect(screen.getByText('MySchema')).toBeInTheDocument()
    })
  })
})
