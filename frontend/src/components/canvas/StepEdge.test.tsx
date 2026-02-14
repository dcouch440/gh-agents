import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { StepEdge } from './StepEdge'

const mockGetBezierPath = vi.fn(() => ['M 0 0 C 50 0, 50 100, 100 100', 50, 50])
const mockDeleteElements = vi.fn(() => Promise.resolve())

vi.mock('@xyflow/react', () => ({
  EdgeLabelRenderer: ({ children }: { children: React.ReactNode }) => <div data-testid="edge-label">{children}</div>,
  getBezierPath: (...args: unknown[]) => mockGetBezierPath(...args),
  useReactFlow: () => ({
    deleteElements: mockDeleteElements,
  }),
  Position: { Left: 'left', Right: 'right' },
}))

vi.mock('./PipeEdgePath', () => ({
  PipeEdgePath: (props: { edgePath: string; color: string; selected: boolean; isProtocol: boolean; animationDirection: string }) => (
    <g
      data-testid="pipe-edge"
      data-color={props.color}
      data-selected={String(props.selected)}
      data-is-protocol={String(props.isProtocol)}
      data-animation-direction={props.animationDirection}
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
    expect(pipe).toHaveAttribute('data-animation-direction', 'normal')
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

  it('calls getBezierPath with source/target coordinates', () => {
    render(
      <svg>
        <StepEdge {...baseProps} />
      </svg>,
    )
    expect(mockGetBezierPath).toHaveBeenCalledWith(
      expect.objectContaining({
        sourceX: 0,
        sourceY: 0,
        targetX: 100,
        targetY: 100,
      }),
    )
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
