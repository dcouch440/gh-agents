import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { ExecutionPanel } from './ExecutionPanel'
import type { StepExecutionState, StepTimelineEvent } from '@/stores'
import type { WorkflowExecutionSummary } from '@/types'

const {
  _activeWorkflowId,
  _runId,
  _isRunning,
  _eventLog,
  _completedStepCount,
  _totalSteps,
  _durationMs,
  _error,
  _startedAt,
  _completedAt,
  _stepStates,
  _viewMode,
  _runs,
  _selectedHistoricalRunId,
  _historicalRun,
  _historyLoading,
  _historyError,
  _fetchRuns,
  _viewHistoricalRun,
  _returnToLive,
} = vi.hoisted(() => ({
  _activeWorkflowId: { value: null as string | null },
  _runId: { value: null as string | null },
  _isRunning: { value: false },
  _eventLog: { value: [] as StepTimelineEvent[] },
  _completedStepCount: { value: 0 },
  _totalSteps: { value: 0 },
  _durationMs: { value: null as number | null },
  _error: { value: null as string | null },
  _startedAt: { value: null as string | null },
  _completedAt: { value: null as string | null },
  _stepStates: { value: {} as Record<string, StepExecutionState> },
  _viewMode: { value: 'live' as 'live' | 'history' },
  _runs: { value: [] as WorkflowExecutionSummary[] },
  _selectedHistoricalRunId: { value: null as string | null },
  _historicalRun: { value: null as WorkflowExecutionSummary | null },
  _historyLoading: { value: false },
  _historyError: { value: null as string | null },
  _fetchRuns: { fn: vi.fn() },
  _viewHistoricalRun: { fn: vi.fn() },
  _returnToLive: { fn: vi.fn() },
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
  workflowStore: {
    store: 'workflow',
    selectActiveWorkflowId: () => _activeWorkflowId.value,
  },
  workflowExecutionStore: {
    store: 'workflowExecution',
    selectRunId: () => _runId.value,
    selectIsRunning: () => _isRunning.value,
    selectEventLog: () => _eventLog.value,
    selectCompletedStepCount: () => _completedStepCount.value,
    selectTotalSteps: () => _totalSteps.value,
    selectDurationMs: () => _durationMs.value,
    selectError: () => _error.value,
    selectStartedAt: () => _startedAt.value,
    selectCompletedAt: () => _completedAt.value,
    selectStepStates: () => _stepStates.value,
    selectViewMode: () => _viewMode.value,
    selectRuns: () => _runs.value,
    selectSelectedHistoricalRunId: () => _selectedHistoricalRunId.value,
    selectHistoricalRun: () => _historicalRun.value,
    selectHistoryLoading: () => _historyLoading.value,
    selectHistoryError: () => _historyError.value,
    fetchRuns: (...args: unknown[]): void => {
      _fetchRuns.fn(...args)
    },
    viewHistoricalRun: (...args: unknown[]): void => {
      _viewHistoricalRun.fn(...args)
    },
    returnToLive: (): void => {
      _returnToLive.fn()
    },
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  _activeWorkflowId.value = null
  _runId.value = null
  _isRunning.value = false
  _eventLog.value = []
  _completedStepCount.value = 0
  _totalSteps.value = 0
  _durationMs.value = null
  _error.value = null
  _startedAt.value = null
  _completedAt.value = null
  _stepStates.value = {}
  _viewMode.value = 'live'
  _runs.value = []
  _selectedHistoricalRunId.value = null
  _historicalRun.value = null
  _historyLoading.value = false
  _historyError.value = null
})

describe('ExecutionPanel', () => {
  it('renders empty state when no runId and no history', () => {
    render(<MemoryRouter><ExecutionPanel /></MemoryRouter>)
    expect(screen.getByText('Run a workflow to see execution details')).toBeInTheDocument()
  })

  it('renders run header and timeline when runId exists in live mode', () => {
    _runId.value = 'run-123'
    _isRunning.value = true
    _totalSteps.value = 3
    _completedStepCount.value = 1
    _startedAt.value = '2025-01-01T00:00:00Z'
    _eventLog.value = [
      { stepId: 's1', stepName: 'Step A', eventType: 'started', ts: '2025-01-01T00:00:01Z' },
      { stepId: 's1', stepName: 'Step A', eventType: 'completed', ts: '2025-01-01T00:00:02Z' },
    ]
    _stepStates.value = {
      s1: {
        status: 'success',
        stepName: 'Step A',
        agentId: null,
        executionId: null,
        output: 'done',
        error: null,
        inputTokens: 10,
        outputTokens: 5,
        durationMs: 100,
        forEachProgress: null,
        subWorkflowProgress: null,
        startedAt: '2025-01-01T00:00:01Z',
        completedAt: '2025-01-01T00:00:02Z',
      },
    }

    render(<MemoryRouter><ExecutionPanel /></MemoryRouter>)
    expect(screen.getByText('Running...')).toBeInTheDocument()
    expect(screen.getByText('1 / 3 steps')).toBeInTheDocument()
    expect(screen.getByText('Step A')).toBeInTheDocument()
  })

  it('renders failed state with error', () => {
    _runId.value = 'run-fail'
    _isRunning.value = false
    _totalSteps.value = 2
    _completedStepCount.value = 1
    _error.value = 'LLM timeout'
    _completedAt.value = '2025-01-01T00:01:00Z'

    render(<MemoryRouter><ExecutionPanel /></MemoryRouter>)
    expect(screen.getByText('Failed')).toBeInTheDocument()
    expect(screen.getByText('LLM timeout')).toBeInTheDocument()
  })

  it('renders completed state', () => {
    _runId.value = 'run-done'
    _isRunning.value = false
    _totalSteps.value = 2
    _completedStepCount.value = 2
    _durationMs.value = 3000
    _completedAt.value = '2025-01-01T00:01:00Z'

    render(<MemoryRouter><ExecutionPanel /></MemoryRouter>)
    expect(screen.getByText('Completed')).toBeInTheDocument()
    expect(screen.getByText('2 / 2 steps')).toBeInTheDocument()
    expect(screen.getByText('3.0s')).toBeInTheDocument()
  })

  it('shows run selector with truncated run ID', () => {
    _runId.value = 'abcdef12-3456-7890'
    render(<MemoryRouter><ExecutionPanel /></MemoryRouter>)
    expect(screen.getByText('Run abcdef12')).toBeInTheDocument()
  })

  it('shows run selector when history exists but no live run', () => {
    _runs.value = [
      {
        id: 'run-old',
        workflow_id: 'wf-1',
        status: 'completed',
        started_at: '2025-01-01T00:00:00Z',
        completed_at: '2025-01-01T00:01:00Z',
        outputs: null,
        error: null,
      },
    ]
    render(<MemoryRouter><ExecutionPanel /></MemoryRouter>)
    expect(screen.queryByText('Run a workflow to see execution details')).not.toBeInTheDocument()
  })

  it('shows error message when history fetch fails', () => {
    _historyError.value = '404 Not Found'
    render(<MemoryRouter><ExecutionPanel /></MemoryRouter>)
    expect(screen.getByText('Failed to load history: 404 Not Found')).toBeInTheDocument()
  })

  it('renders historical run summary in history mode', () => {
    _viewMode.value = 'history'
    _historicalRun.value = {
      id: 'run-hist',
      workflow_id: 'wf-1',
      status: 'completed',
      started_at: '2025-01-01T00:00:00Z',
      completed_at: '2025-01-01T00:00:10Z',
      outputs: { '': { response: 'Hello' } },
      error: null,
    }
    _runs.value = [_historicalRun.value]
    _selectedHistoricalRunId.value = 'run-hist'

    render(<MemoryRouter><ExecutionPanel /></MemoryRouter>)
    // Should not show the live timeline
    expect(screen.queryByText('Running...')).not.toBeInTheDocument()
  })
})
