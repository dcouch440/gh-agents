import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { LastRunTab } from './LastRunTab'
import type { StepLastRunResponse } from '@/types'

// ── Mocks ────────────────────────────────────────────────────────────────────

const mockGetStepLastRun = vi.hoisted(() => vi.fn())

vi.mock('@/api', () => ({
  api: {
    workflows: {
      getStepLastRun: mockGetStepLastRun,
    },
  },
}))

const mockSelectActiveWorkflowId = vi.hoisted(() => vi.fn(() => (s: { activeWorkflowId: string | null }) => s.activeWorkflowId))

vi.mock('@/stores', () => ({
  useStore: (_store: unknown, selector: (s: Record<string, unknown>) => unknown) =>
    selector({ activeWorkflowId: 'wf-001' }),
  workflowStore: {
    store: {},
    selectActiveWorkflowId: mockSelectActiveWorkflowId(),
  },
}))

// ── Fixtures ─────────────────────────────────────────────────────────────────

const mockLastRun: StepLastRunResponse = {
  execution_id: 'exec-1',
  workflow_execution_id: 'wf-exec-1',
  status: 'completed',
  started_at: '2025-01-01T00:00:00Z',
  completed_at: '2025-01-01T00:01:00Z',
  duration_ms: 60000,
  output: 'Step output here',
  structured_output: null,
  input_tokens: 1500,
  output_tokens: 800,
  cost_usd: 0.025,
  error: null,
  phases: null,
}

const mockLastRunWithPhases: StepLastRunResponse = {
  ...mockLastRun,
  output: null,
  phases: [
    {
      id: 'phase-1',
      phase: 'strategy',
      document_name: null,
      status: 'completed',
      output_content: 'Strategy output content',
      input_tokens: 500,
      output_tokens: 200,
      cost_usd: 0.005,
      model: 'claude-sonnet-4',
      started_at: '2025-01-01T00:00:00Z',
      completed_at: '2025-01-01T00:00:10Z',
      error_message: null,
    },
    {
      id: 'phase-2',
      phase: 'research',
      document_name: 'API Reference',
      status: 'completed',
      output_content: 'Research findings',
      input_tokens: 800,
      output_tokens: 400,
      cost_usd: 0.012,
      model: 'claude-sonnet-4',
      started_at: '2025-01-01T00:00:10Z',
      completed_at: '2025-01-01T00:00:30Z',
      error_message: null,
    },
  ],
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('LastRunTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows loading spinner while fetching', () => {
    // Never resolves
    mockGetStepLastRun.mockReturnValue(new Promise(() => {}))
    render(<LastRunTab stepId="step-1" />)
    expect(screen.getByRole('progressbar')).toBeInTheDocument()
  })

  it('shows empty state when API returns 404', async () => {
    mockGetStepLastRun.mockRejectedValueOnce(new Error('404 Not Found'))
    render(<LastRunTab stepId="step-1" />)

    await waitFor(() => {
      expect(screen.getByText(/No execution data yet/)).toBeInTheDocument()
    })
  })

  it('shows error message on non-404 failure', async () => {
    mockGetStepLastRun.mockRejectedValueOnce(new Error('Server error'))
    render(<LastRunTab stepId="step-1" />)

    await waitFor(() => {
      expect(screen.getByText('Server error')).toBeInTheDocument()
    })
  })

  it('renders status, duration, tokens, and cost for a completed run', async () => {
    mockGetStepLastRun.mockResolvedValueOnce(mockLastRun)
    render(<LastRunTab stepId="step-1" />)

    await waitFor(() => {
      expect(screen.getByText('completed')).toBeInTheDocument()
    })

    expect(screen.getByText('1.0m')).toBeInTheDocument()
    expect(screen.getByText('1.5k in / 800 out')).toBeInTheDocument()
    expect(screen.getByText('$0.03')).toBeInTheDocument()
  })

  it('renders output section for steps without phases', async () => {
    mockGetStepLastRun.mockResolvedValueOnce(mockLastRun)
    render(<LastRunTab stepId="step-1" />)

    await waitFor(() => {
      expect(screen.getByText('Step output here')).toBeInTheDocument()
    })
  })

  it('renders pipeline phases for steps with phases', async () => {
    mockGetStepLastRun.mockResolvedValueOnce(mockLastRunWithPhases)
    render(<LastRunTab stepId="step-1" />)

    await waitFor(() => {
      expect(screen.getByText(/Pipeline Phases/)).toBeInTheDocument()
    })

    expect(screen.getByText('strategy')).toBeInTheDocument()
    expect(screen.getByText('research: API Reference')).toBeInTheDocument()
  })

  it('calls getStepLastRun with correct IDs', async () => {
    mockGetStepLastRun.mockResolvedValueOnce(mockLastRun)
    render(<LastRunTab stepId="step-42" />)

    await waitFor(() => {
      expect(mockGetStepLastRun).toHaveBeenCalledWith('wf-001', 'step-42')
    })
  })
})
