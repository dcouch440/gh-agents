import { describe, it, expect, vi } from 'vitest'
import { render } from '@testing-library/react'
import { StepEdge } from './StepEdge'

const mockGetBezierPath = vi.fn(() => ['M 0 0 C 50 0, 50 100, 100 100', 50, 50])

vi.mock('@xyflow/react', () => ({
  BaseEdge: ({ path, style }: { path: string; style: React.CSSProperties }) => (
    <svg>
      <path data-testid="edge-path" d={path} style={style} />
    </svg>
  ),
  getBezierPath: (...args: unknown[]) => mockGetBezierPath(...args),
  Position: { Left: 'left', Right: 'right' },
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
  data: { condition: null },
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

describe('StepEdge', () => {
  it('renders an edge path', () => {
    const { container } = render(
      <svg>
        <StepEdge {...baseProps} />
      </svg>,
    )
    const path = container.querySelector('[data-testid="edge-path"]')
    expect(path).toBeInTheDocument()
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

  it('uses lower opacity when not selected', () => {
    const { container } = render(
      <svg>
        <StepEdge {...baseProps} selected={false} />
      </svg>,
    )
    const path = container.querySelector('[data-testid="edge-path"]')
    expect(path).toHaveStyle({ opacity: '0.4' })
  })

  it('uses higher opacity when selected', () => {
    const { container } = render(
      <svg>
        <StepEdge {...baseProps} selected={true} />
      </svg>,
    )
    const path = container.querySelector('[data-testid="edge-path"]')
    expect(path).toHaveStyle({ opacity: '0.8' })
  })
})
