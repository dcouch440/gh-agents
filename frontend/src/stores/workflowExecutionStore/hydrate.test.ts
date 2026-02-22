import { workflowExecutionStore } from '.'
import {
  mapApiStatusToStoreStatus,
  mapRunStepToStepState,
  buildSubWorkflowProgress,
  buildEventLog,
  hydrateLatestRun,
} from './hydrate'
import type { RunStepResult, WorkflowExecutionSummary, RunDetailResponse } from '@/types'

const { mockListExecutions, mockGetRunDetail } = vi.hoisted(() => ({
  mockListExecutions: vi.fn(),
  mockGetRunDetail: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    workflows: {
      listExecutions: mockListExecutions,
      getRunDetail: mockGetRunDetail,
    },
  },
}))

const getState = () => workflowExecutionStore.store.getState()

const makeSummary = (overrides: Partial<WorkflowExecutionSummary> = {}): WorkflowExecutionSummary => ({
  id: 'run-1',
  workflow_id: 'w1',
  status: 'completed',
  started_at: '2025-01-01T00:00:00Z',
  completed_at: '2025-01-01T00:01:00Z',
  outputs: null,
  error: null,
  execution_mode: 'standalone',
  template_id: null,
  ...overrides,
})

const makeStep = (overrides: Partial<RunStepResult> = {}): RunStepResult => ({
  step_id: 's1',
  step_name: 'Step One',
  execution_mode: 'single',
  execution_id: 'exec-1',
  status: 'completed',
  started_at: '2025-01-01T00:00:10Z',
  completed_at: '2025-01-01T00:00:30Z',
  duration_ms: 20000,
  output: 'result text',
  structured_output: null,
  input_tokens: 100,
  output_tokens: 50,
  cost_usd: 0.01,
  error: null,
  phases: null,
  child_execution_id: null,
  child_steps: null,
  ...overrides,
})

const makeDetail = (steps: RunStepResult[], summary: WorkflowExecutionSummary): RunDetailResponse => ({
  execution: summary,
  steps,
  total_input_tokens: steps.reduce((sum, s) => sum + (s.input_tokens ?? 0), 0),
  total_output_tokens: steps.reduce((sum, s) => sum + (s.output_tokens ?? 0), 0),
  total_cost_usd: steps.reduce((sum, s) => sum + (s.cost_usd ?? 0), 0),
  duration_ms: 60000,
  template_name: null,
})

beforeEach(() => {
  vi.clearAllMocks()
  workflowExecutionStore.reset()
})

describe('mapApiStatusToStoreStatus', () => {
  it.each([
    ['completed', 'success'],
    ['failed', 'error'],
    ['skipped', 'skipped'],
    ['running', 'running'],
    ['pending', 'pending'],
    ['paused', 'paused'],
    ['unknown', 'idle'],
    ['', 'idle'],
  ])('maps "%s" to "%s"', (input, expected) => {
    expect(mapApiStatusToStoreStatus(input)).toBe(expected)
  })
})

describe('mapRunStepToStepState', () => {
  it('maps a completed step', () => {
    const step = makeStep()
    const state = mapRunStepToStepState(step)

    expect(state.status).toBe('success')
    expect(state.stepName).toBe('Step One')
    expect(state.executionId).toBe('exec-1')
    expect(state.output).toBe('result text')
    expect(state.inputTokens).toBe(100)
    expect(state.outputTokens).toBe(50)
    expect(state.durationMs).toBe(20000)
    expect(state.error).toBeNull()
    expect(state.agentId).toBeNull()
    expect(state.forEachProgress).toBeNull()
    expect(state.subWorkflowProgress).toBeNull()
    expect(state.startedAt).toBe('2025-01-01T00:00:10Z')
    expect(state.completedAt).toBe('2025-01-01T00:00:30Z')
  })

  it('maps a failed step', () => {
    const step = makeStep({ status: 'failed', error: 'timeout', output: null })
    const state = mapRunStepToStepState(step)

    expect(state.status).toBe('error')
    expect(state.error).toBe('timeout')
  })

  it('maps a skipped step', () => {
    const step = makeStep({
      status: 'skipped',
      execution_id: null,
      started_at: null,
      completed_at: null,
      output: null,
      input_tokens: null,
      output_tokens: null,
      duration_ms: null,
    })
    const state = mapRunStepToStepState(step)

    expect(state.status).toBe('skipped')
    expect(state.executionId).toBeNull()
  })
})

describe('buildSubWorkflowProgress', () => {
  it('returns null when child_execution_id is null', () => {
    const step = makeStep({ child_execution_id: null, child_steps: null })
    expect(buildSubWorkflowProgress(step)).toBeNull()
  })

  it('returns null when child_steps is empty', () => {
    const step = makeStep({ child_execution_id: 'ce-1', child_steps: [] })
    expect(buildSubWorkflowProgress(step)).toBeNull()
  })

  it('builds progress from child steps', () => {
    const step = makeStep({
      status: 'completed',
      child_execution_id: 'ce-1',
      child_steps: [
        { step_name: 'Designer', execution_mode: 'single', status: 'completed', input_tokens: 50, output_tokens: 25, duration_ms: 1000, error: null },
        { step_name: 'Agent 1', execution_mode: 'single', status: 'completed', input_tokens: 80, output_tokens: 40, duration_ms: 2000, error: null },
      ],
    })

    const progress = buildSubWorkflowProgress(step)
    expect(progress).not.toBeNull()
    expect(progress!.childExecutionId).toBe('ce-1')
    expect(progress!.totalSteps).toBe(2)
    expect(progress!.completedSteps).toBe(2)
    expect(progress!.status).toBe('completed')
    expect(progress!.childSteps).toHaveLength(2)
    expect(progress!.childSteps[0].childStepName).toBe('Designer')
    expect(progress!.childSteps[0].status).toBe('success')
    expect(progress!.childSteps[1].status).toBe('success')
  })

  it('counts failed child steps as terminal', () => {
    const step = makeStep({
      status: 'failed',
      child_execution_id: 'ce-1',
      child_steps: [
        { step_name: 'Designer', execution_mode: 'single', status: 'completed', input_tokens: 50, output_tokens: 25, duration_ms: 1000, error: null },
        { step_name: 'Agent 1', execution_mode: 'single', status: 'failed', input_tokens: null, output_tokens: null, duration_ms: null, error: 'boom' },
      ],
    })

    const progress = buildSubWorkflowProgress(step)
    expect(progress!.completedSteps).toBe(2)
    expect(progress!.status).toBe('failed')
    expect(progress!.childSteps[1].status).toBe('error')
    expect(progress!.childSteps[1].error).toBe('boom')
  })
})

describe('buildEventLog', () => {
  it('returns empty for empty steps', () => {
    expect(buildEventLog([])).toEqual([])
  })

  it('builds started + completed events sorted by time', () => {
    const steps = [
      makeStep({ step_id: 's1', step_name: 'A', started_at: '2025-01-01T00:00:01Z', completed_at: '2025-01-01T00:00:05Z', status: 'completed' }),
      makeStep({ step_id: 's2', step_name: 'B', started_at: '2025-01-01T00:00:03Z', completed_at: '2025-01-01T00:00:08Z', status: 'completed' }),
    ]

    const events = buildEventLog(steps)
    expect(events).toHaveLength(4)
    expect(events[0]).toEqual({ stepId: 's1', stepName: 'A', eventType: 'started', ts: '2025-01-01T00:00:01Z' })
    expect(events[1]).toEqual({ stepId: 's2', stepName: 'B', eventType: 'started', ts: '2025-01-01T00:00:03Z' })
    expect(events[2]).toEqual({ stepId: 's1', stepName: 'A', eventType: 'completed', ts: '2025-01-01T00:00:05Z' })
    expect(events[3]).toEqual({ stepId: 's2', stepName: 'B', eventType: 'completed', ts: '2025-01-01T00:00:08Z' })
  })

  it('emits failed event type for failed steps', () => {
    const steps = [
      makeStep({ step_id: 's1', step_name: 'A', started_at: '2025-01-01T00:00:01Z', completed_at: '2025-01-01T00:00:05Z', status: 'failed' }),
    ]

    const events = buildEventLog(steps)
    expect(events).toHaveLength(2)
    expect(events[1].eventType).toBe('failed')
  })

  it('skips terminal event for running steps', () => {
    const steps = [
      makeStep({ step_id: 's1', step_name: 'A', started_at: '2025-01-01T00:00:01Z', completed_at: null, status: 'running' }),
    ]

    const events = buildEventLog(steps)
    expect(events).toHaveLength(1)
    expect(events[0].eventType).toBe('started')
  })

  it('skips steps with no started_at', () => {
    const steps = [
      makeStep({ step_id: 's1', step_name: 'A', started_at: null, completed_at: null, status: 'skipped' }),
    ]

    const events = buildEventLog(steps)
    expect(events).toEqual([])
  })
})

describe('hydrateLatestRun', () => {
  it('does nothing when no runs exist', async () => {
    mockListExecutions.mockResolvedValue([])

    await hydrateLatestRun('w1')

    expect(getState().runId).toBeNull()
    expect(getState().runs).toEqual([])
    expect(mockGetRunDetail).not.toHaveBeenCalled()
  })

  it('hydrates store with latest completed run', async () => {
    const summary = makeSummary()
    const steps = [
      makeStep({ step_id: 's1', step_name: 'A', status: 'completed' }),
      makeStep({ step_id: 's2', step_name: 'B', status: 'completed', started_at: '2025-01-01T00:00:20Z', completed_at: '2025-01-01T00:00:40Z' }),
    ]
    mockListExecutions.mockResolvedValue([summary])
    mockGetRunDetail.mockResolvedValue(makeDetail(steps, summary))

    await hydrateLatestRun('w1')

    const s = getState()
    expect(s.runId).toBe('run-1')
    expect(s.workflowId).toBe('w1')
    expect(s.isRunning).toBe(false)
    expect(s.stepStates['s1'].status).toBe('success')
    expect(s.stepStates['s2'].status).toBe('success')
    expect(s.totalSteps).toBe(2)
    expect(s.completedStepCount).toBe(2)
    expect(s.durationMs).toBe(60000)
    expect(s.startedAt).toBe('2025-01-01T00:00:00Z')
    expect(s.completedAt).toBe('2025-01-01T00:01:00Z')
    expect(s.viewMode).toBe('live')
    expect(s.runs).toHaveLength(1)
  })

  it('hydrates a running execution with isRunning=true', async () => {
    const summary = makeSummary({ status: 'running', completed_at: null })
    const steps = [
      makeStep({ step_id: 's1', step_name: 'A', status: 'completed' }),
      makeStep({ step_id: 's2', step_name: 'B', status: 'running', completed_at: null, output: null }),
    ]
    mockListExecutions.mockResolvedValue([summary])
    mockGetRunDetail.mockResolvedValue(makeDetail(steps, summary))

    await hydrateLatestRun('w1')

    const s = getState()
    expect(s.isRunning).toBe(true)
    expect(s.stepStates['s1'].status).toBe('success')
    expect(s.stepStates['s2'].status).toBe('running')
    expect(s.completedStepCount).toBe(1)
  })

  it('hydrates eventLog from step data', async () => {
    const summary = makeSummary()
    const steps = [
      makeStep({ step_id: 's1', step_name: 'A', started_at: '2025-01-01T00:00:05Z', completed_at: '2025-01-01T00:00:15Z', status: 'completed' }),
    ]
    mockListExecutions.mockResolvedValue([summary])
    mockGetRunDetail.mockResolvedValue(makeDetail(steps, summary))

    await hydrateLatestRun('w1')

    const log = getState().eventLog
    expect(log).toHaveLength(2)
    expect(log[0].eventType).toBe('started')
    expect(log[1].eventType).toBe('completed')
  })

  it('skips hydration when runId already set (WS event arrived first)', async () => {
    workflowExecutionStore.store.setState({ runId: 'ws-run' })
    mockListExecutions.mockResolvedValue([makeSummary()])

    await hydrateLatestRun('w1')

    expect(getState().runId).toBe('ws-run')
    expect(mockGetRunDetail).not.toHaveBeenCalled()
  })

  it('skips hydration when runId becomes set during fetch (race condition)', async () => {
    const summary = makeSummary()
    mockListExecutions.mockResolvedValue([summary])
    mockGetRunDetail.mockImplementation(() => {
      // Simulate WS event arriving during the API call
      workflowExecutionStore.store.setState({ runId: 'ws-run-late' })
      return Promise.resolve(makeDetail([makeStep()], summary))
    })

    await hydrateLatestRun('w1')

    // Should have bailed — runId stays as the WS-set value
    expect(getState().runId).toBe('ws-run-late')
    expect(getState().stepStates).toEqual({})
  })

  it('handles API error gracefully', async () => {
    mockListExecutions.mockRejectedValue(new Error('network error'))
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})

    await hydrateLatestRun('w1')

    expect(getState().runId).toBeNull()
    spy.mockRestore()
  })

  it('handles run with mixed step statuses', async () => {
    const summary = makeSummary({ status: 'failed', error: 'step failed' })
    const steps = [
      makeStep({ step_id: 's1', step_name: 'A', status: 'completed' }),
      makeStep({ step_id: 's2', step_name: 'B', status: 'failed', error: 'timeout' }),
      makeStep({ step_id: 's3', step_name: 'C', status: 'skipped', started_at: null, completed_at: null, output: null, execution_id: null }),
    ]
    mockListExecutions.mockResolvedValue([summary])
    mockGetRunDetail.mockResolvedValue(makeDetail(steps, summary))

    await hydrateLatestRun('w1')

    const s = getState()
    expect(s.isRunning).toBe(false)
    expect(s.error).toBe('step failed')
    expect(s.stepStates['s1'].status).toBe('success')
    expect(s.stepStates['s2'].status).toBe('error')
    expect(s.stepStates['s2'].error).toBe('timeout')
    expect(s.stepStates['s3'].status).toBe('skipped')
    expect(s.completedStepCount).toBe(3)
  })

  it('populates runs even when detail fetch fails', async () => {
    const summary = makeSummary()
    mockListExecutions.mockResolvedValue([summary])
    mockGetRunDetail.mockRejectedValue(new Error('detail fetch failed'))
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})

    await hydrateLatestRun('w1')

    // runs should be set from listExecutions, but stepStates should not be hydrated
    expect(getState().runs).toHaveLength(1)
    expect(getState().runId).toBeNull()
    spy.mockRestore()
  })
})
