import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { StepEdge } from './StepEdge'

const mockDeleteElements = vi.fn(() => Promise.resolve())
const mockComputeBezierPath = vi.fn(() => 'M 0 0 C 50 0 50 100 100 100')
const mockComputeBezierLabel = vi.fn(() => ({ labelX: 50, labelY: 50 }))

vi.mock('@xyflow/react', () => ({
  EdgeLabelRenderer: ({ children }: { children: React.ReactNode }) => <div data-testid="edge-label">{children}</div>,
  useReactFlow: () => ({
    deleteElements: mockDeleteElements,
  }),
  Position: { Left: 'left', Right: 'right' },
}))

vi.mock('./edges/bezierPath', () => ({
  computeBezierPath: (...args: unknown[]) => mockComputeBezierPath(...args),
  computeBezierLabel: (...args: unknown[]) => mockComputeBezierLabel(...args),
}))

vi.mock('./useCanvasLOD', () => ({ useCanvasLOD: () => 'full' }))

vi.mock('./PipeEdgePath', () => ({
  PipeEdgePath: (props: { edgePath: string; color: string; selected: boolean; isProtocol: boolean }) => (
    <g
      data-testid="pipe-edge"
      data-color={props.color}
      data-selected={String(props.selected)}
      data-is-protocol={String(props.isProtocol)}
    />
  ),
}))

const baseProps = {
  id: 'edge-001',
  source: 'step-001',
  target: 'step-002',
  sourceX: 0,
  sourceY: 0,
  targetX: 100,
  targetY: 100,
  sourcePosition: 'right' as const,
  targetPosition: 'left' as const,
  selected: false,
  animated: false,
  data: { sourceColor: '#3b82f6', isProtocolEdge: false },
  interactionWidth: 20,
  sourceHandleId: null,
  targetHandleId: null,
  markerStart: undefined,
  markerEnd: undefined,
  pathOptions: undefined,
  style: {},
  label: undefined,
  labelStyle: undefined,
  labelShowBg: undefined,
  labelBgStyle: undefined,
  labelBgPadding: undefined,
  labelBgBorderRadius: undefined,
  deletable: true,
  selectable: true,
  focusable: true,
}

beforeEach(() => {
  vi.clearAllMocks()
})

describe('StepEdge', () => {
  it('renders PipeEdgePath with correct props', () => {
    render(
      <svg>
        <StepEdge {...baseProps} />
      </svg>,
    )
    const pipe = screen.getByTestId('pipe-edge')
    expect(pipe).toBeInTheDocument()
    expect(pipe).toHaveAttribute('data-selected', 'false')
    expect(pipe).toHaveAttribute('data-is-protocol', 'false')
  })

  it('passes source color and protocol flag when present', () => {
    render(
      <svg>
        <StepEdge {...baseProps} data={{ sourceColor: '#3b82f6', isProtocolEdge: true }} />
      </svg>,
    )
    const pipe = screen.getByTestId('pipe-edge')
    expect(pipe).toHaveAttribute('data-color', '#3b82f6')
    expect(pipe).toHaveAttribute('data-is-protocol', 'true')
  })

  it('passes selected state to PipeEdgePath', () => {
    render(
      <svg>
        <StepEdge {...baseProps} selected={true} />
      </svg>,
    )
    const pipe = screen.getByTestId('pipe-edge')
    expect(pipe).toHaveAttribute('data-selected', 'true')
  })

  it('calls computeBezierPath with source/target coordinates', () => {
    render(
      <svg>
        <StepEdge {...baseProps} />
      </svg>,
    )
    expect(mockComputeBezierPath).toHaveBeenCalledWith(0, 0, 100, 100, 'right', 'left')
  })

  it('renders delete button', () => {
    render(
      <svg>
        <StepEdge {...baseProps} />
      </svg>,
    )
    expect(screen.getByTestId('edge-label')).toBeInTheDocument()
  })

  it('calls deleteElements when delete button is clicked', async () => {
    const user = userEvent.setup()
    render(
      <svg>
        <StepEdge {...baseProps} />
      </svg>,
    )

    const deleteButton = screen.getByRole('button')
    await user.click(deleteButton)

    expect(mockDeleteElements).toHaveBeenCalledWith({ edges: [{ id: 'edge-001' }] })
  })
})
