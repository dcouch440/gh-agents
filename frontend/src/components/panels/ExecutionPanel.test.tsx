import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ExecutionPanel } from './ExecutionPanel'
import type { StepExecutionState, StepTimelineEvent } from '@/stores'

const {
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
} = vi.hoisted(() => ({
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
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (typeof selector === 'function') return (selector as (s: unknown) => unknown)(null)
    return undefined
  }),
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
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
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
})

describe('ExecutionPanel', () => {
  it('renders empty state when no runId', () => {
    render(<ExecutionPanel />)
    expect(screen.getByText('Run a workflow to see execution details')).toBeInTheDocument()
  })

  it('renders run header and timeline when runId exists', () => {
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
        startedAt: '2025-01-01T00:00:01Z',
        completedAt: '2025-01-01T00:00:02Z',
      },
    }

    render(<ExecutionPanel />)
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

    render(<ExecutionPanel />)
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

    render(<ExecutionPanel />)
    expect(screen.getByText('Completed')).toBeInTheDocument()
    expect(screen.getByText('2 / 2 steps')).toBeInTheDocument()
    expect(screen.getByText('3.0s')).toBeInTheDocument()
  })

  it('shows run selector with truncated run ID', () => {
    _runId.value = 'abcdef12-3456-7890'
    render(<ExecutionPanel />)
    expect(screen.getByText('Run abcdef12')).toBeInTheDocument()
  })
})
