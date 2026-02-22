import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { DispatchTab } from './DispatchTab'
import type { DispatchEntry } from '@/stores/dispatchStore'

const mockEntry: DispatchEntry = {
  executionId: 'exec-1',
  stepId: 'step-1',
  status: 'running',
  instruction: 'Set up the search agent',
  message: null,
  summary: null,
  error: null,
  startedAt: '2025-01-01T00:00:00Z',
  trace: [
    { type: 'token', content: 'Working on it...', ts: '2025-01-01T00:00:01Z' },
  ],
  tokenBuffer: 'Working on it...',
}

const mockSelectByStepId = vi.fn()
const mockSelectActiveWorkflowId = vi.fn()

vi.mock('@/stores', () => ({
  useStore: (_store: unknown, selector: (s: unknown) => unknown) => selector({ byStep: {} }),
  dispatchStore: {
    store: {},
    selectByStepId: (stepId: string) => mockSelectByStepId(stepId) as (s: unknown) => DispatchEntry | null,
    hydrateFromApi: vi.fn(),
  },
  workflowStore: {
    store: {},
    selectActiveWorkflowId: () => mockSelectActiveWorkflowId() as string | null,
  },
}))

vi.mock('@/api', () => ({
  api: {
    workflows: {
      getStepDispatchHistory: vi.fn().mockRejectedValue(new Error('not found')),
    },
  },
}))

vi.mock('./DispatchTraceView', () => ({
  DispatchTraceView: ({ entry }: { entry: DispatchEntry }) => (
    <div data-testid="trace-view">{entry.status}</div>
  ),
}))

describe('DispatchTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSelectActiveWorkflowId.mockReturnValue('wf-1')
  })

  it('renders empty state when no dispatch entry exists', () => {
    mockSelectByStepId.mockReturnValue(() => null)
    render(<DispatchTab stepId="step-1" />)
    expect(screen.getByText('No dispatch activity yet.')).toBeInTheDocument()
  })

  it('renders instruction header when instruction is present', () => {
    mockSelectByStepId.mockReturnValue(() => mockEntry)
    render(<DispatchTab stepId="step-1" />)
    expect(screen.getByText('Set up the search agent')).toBeInTheDocument()
  })

  it('renders DispatchTraceView when entry exists', () => {
    mockSelectByStepId.mockReturnValue(() => mockEntry)
    render(<DispatchTab stepId="step-1" />)
    expect(screen.getByTestId('trace-view')).toHaveTextContent('running')
  })

  it('renders completed entry', () => {
    const completedEntry = { ...mockEntry, status: 'completed' as const }
    mockSelectByStepId.mockReturnValue(() => completedEntry)
    render(<DispatchTab stepId="step-1" />)
    expect(screen.getByTestId('trace-view')).toHaveTextContent('completed')
  })
})
