import { workflowExecutionStore } from '.'
import {
  mapApiStatusToStoreStatus,
  mapRunStepToStepState,
  buildEventLog,
  applyServerRun,
  setActiveRun,
  beginRun,
  preferLocal,
} from './hydrate'
import type { ServerRunSnapshot } from './hydrate'
import type { RunStepResult, WorkflowExecutionSummary } from '@/types'

const { mockGetWorkshopStatus } = vi.hoisted(() => ({
  mockGetWorkshopStatus: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    workflows: {
      getWorkshopStatus: mockGetWorkshopStatus,
    },
  },
}))

const getState = () => workflowExecutionStore.store.getState()

/** Assert a step exists before reading it — `noUncheckedIndexedAccess` is on. */
const stepState = (stepId: string) => {
  const state = getState().stepStates[stepId]
  if (state === undefined) throw new Error(`no step state for ${stepId}`)
  return state
}

/** Assert an event log entry exists at `index`. */
const logEntry = (index: number) => {
  const entry = getState().eventLog[index]
  if (entry === undefined) throw new Error(`no event log entry at ${String(index)}`)
  return entry
}

const makeSummary = (overrides: Partial<WorkflowExecutionSummary> = {}): WorkflowExecutionSummary => ({
  id: 'run-1',
  workflow_id: 'w1',
  status: 'completed',
  started_at: '2025-01-01T00:00:00Z',
  completed_at: '2025-01-01T00:01:00Z',
  outputs: null,
  error: null,
  execution_mode: 'single',
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
  ...overrides,
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
    expect(events[1]?.eventType).toBe('failed')
  })

  it('skips terminal event for running steps', () => {
    const steps = [
      makeStep({ step_id: 's1', step_name: 'A', started_at: '2025-01-01T00:00:01Z', completed_at: null, status: 'running' }),
    ]

    const events = buildEventLog(steps)
    expect(events).toHaveLength(1)
    expect(events[0]?.eventType).toBe('started')
  })

  it('skips steps with no started_at', () => {
    const steps = [
      makeStep({ step_id: 's1', step_name: 'A', started_at: null, completed_at: null, status: 'skipped' }),
    ]

    const events = buildEventLog(steps)
    expect(events).toEqual([])
  })
})

const makeSnapshot = (
  steps: RunStepResult[],
  overrides: Partial<ServerRunSnapshot> = {},
): ServerRunSnapshot => ({
  runId: 'run-1',
  workflowId: 'w1',
  status: 'completed',
  startedAt: '2025-01-01T00:00:00Z',
  completedAt: '2025-01-01T00:01:00Z',
  durationMs: 60000,
  error: null,
  steps,
  ...overrides,
})

describe('applyServerRun', () => {
  it('clears the overlay when the workflow has never run', () => {
    applyServerRun(makeSnapshot([], { runId: null }))

    expect(getState().runId).toBeNull()
    expect(getState().stepStates).toEqual({})
    expect(getState().workflowId).toBe('w1')
  })

  it('hydrates the latest completed run', () => {
    applyServerRun(makeSnapshot([
      makeStep({ step_id: 's1', step_name: 'A', status: 'completed' }),
      makeStep({ step_id: 's2', step_name: 'B', status: 'completed', started_at: '2025-01-01T00:00:20Z', completed_at: '2025-01-01T00:00:40Z' }),
    ]))

    const s = getState()
    expect(s.runId).toBe('run-1')
    expect(s.workflowId).toBe('w1')
    expect(s.isRunning).toBe(false)
    expect(stepState('s1').status).toBe('success')
    expect(stepState('s2').status).toBe('success')
    expect(s.totalSteps).toBe(2)
    expect(s.completedStepCount).toBe(2)
    expect(s.durationMs).toBe(60000)
    expect(s.viewMode).toBe('live')
  })

  it('marks a running execution as running', () => {
    applyServerRun(makeSnapshot([
      makeStep({ step_id: 's1', status: 'completed' }),
      makeStep({ step_id: 's2', status: 'running', completed_at: null, output: null }),
    ], { status: 'running', completedAt: null }))

    const s = getState()
    expect(s.isRunning).toBe(true)
    expect(stepState('s2').status).toBe('running')
    expect(s.completedStepCount).toBe(1)
  })

  it('treats a pending run as running, so a just-started run is not missed', () => {
    // A freshly created run has status 'pending' and no started_at. Before this,
    // it sorted last in the history list and the UI showed a stale run instead.
    applyServerRun(makeSnapshot([], { runId: 'run-new', status: 'pending', startedAt: null, completedAt: null }))

    expect(getState().runId).toBe('run-new')
    expect(getState().isRunning).toBe(true)
  })

  it('builds the event log from step timestamps', () => {
    applyServerRun(makeSnapshot([
      makeStep({ step_id: 's1', started_at: '2025-01-01T00:00:05Z', completed_at: '2025-01-01T00:00:15Z', status: 'completed' }),
    ]))

    expect(getState().eventLog).toHaveLength(2)
    expect(logEntry(0).eventType).toBe('started')
    expect(logEntry(1).eventType).toBe('completed')
  })

  it('applies even when a WS event already set a runId — no more silent skip', () => {
    // The old race guard bailed out here, leaving the tree empty after a refresh.
    workflowExecutionStore.store.setState({ runId: 'run-1' })

    applyServerRun(makeSnapshot([makeStep({ step_id: 's1', status: 'completed' })]))

    expect(getState().runId).toBe('run-1')
    expect(stepState('s1').status).toBe('success')
  })

  it('swaps the overlay when the server reports a different run', () => {
    applyServerRun(makeSnapshot([makeStep({ step_id: 'old-step', status: 'completed' })]))
    workflowExecutionStore.store.setState({ runs: [makeSummary()] })

    applyServerRun(makeSnapshot([makeStep({ step_id: 'new-step', status: 'running', completed_at: null })], {
      runId: 'run-2',
      status: 'running',
      completedAt: null,
    }))

    const s = getState()
    expect(s.runId).toBe('run-2')
    expect(getState().stepStates['old-step']).toBeUndefined()
    expect(stepState('new-step').status).toBe('running')
    // History is a separate layer and must survive the swap.
    expect(s.runs).toHaveLength(1)
  })

  it('records mixed step statuses and the run error', () => {
    applyServerRun(makeSnapshot([
      makeStep({ step_id: 's1', status: 'completed' }),
      makeStep({ step_id: 's2', status: 'failed', error: 'timeout' }),
      makeStep({ step_id: 's3', status: 'skipped', started_at: null, completed_at: null, output: null, execution_id: null }),
    ], { status: 'failed', error: 'step failed' }))

    const s = getState()
    expect(s.isRunning).toBe(false)
    expect(s.error).toBe('step failed')
    expect(stepState('s2').error).toBe('timeout')
    expect(stepState('s3').status).toBe('skipped')
    expect(s.completedStepCount).toBe(3)
  })

  it('shows unreached steps of an in-flight run as pending, not skipped', () => {
    // A just-started run has no execution rows yet, so the server reports every
    // step as "skipped". Reading that literally made a fresh run look finished.
    applyServerRun(makeSnapshot([
      makeStep({ step_id: 's1', status: 'running', completed_at: null, execution_id: 'exec-1' }),
      makeStep({ step_id: 's2', status: 'skipped', started_at: null, completed_at: null, output: null, execution_id: null }),
    ], { status: 'running', completedAt: null }))

    expect(stepState('s2').status).toBe('pending')
    expect(getState().completedStepCount).toBe(0)
  })

  it('keeps genuinely skipped steps skipped once the run has finished', () => {
    applyServerRun(makeSnapshot([
      makeStep({ step_id: 's1', status: 'completed' }),
      makeStep({ step_id: 's2', status: 'skipped', started_at: null, completed_at: null, output: null, execution_id: null }),
    ]))

    expect(stepState('s2').status).toBe('skipped')
  })

  it('is idempotent across repeated polls', () => {
    const snapshot = makeSnapshot([makeStep({ step_id: 's1', status: 'completed' })])
    applyServerRun(snapshot)
    const first = getState()
    applyServerRun(snapshot)
    const second = getState()

    expect(second.runId).toBe(first.runId)
    expect(second.completedStepCount).toBe(first.completedStepCount)
    expect(second.eventLog).toHaveLength(first.eventLog.length)
  })
})

describe('preferLocal', () => {
  it('keeps a locally running step over a server pending one', () => {
    const local = mapRunStepToStepState(makeStep({ status: 'running', completed_at: null }))
    const server = mapRunStepToStepState(makeStep({ status: 'pending', started_at: null, completed_at: null }))

    expect(preferLocal(local, server)).toBe(true)
  })

  it('takes the server state when there is no local state', () => {
    const server = mapRunStepToStepState(makeStep())
    expect(preferLocal(undefined, server)).toBe(false)
  })

  it('keeps whichever side progressed further', () => {
    const older = mapRunStepToStepState(makeStep({ completed_at: '2025-01-01T00:00:10Z' }))
    const newer = mapRunStepToStepState(makeStep({ completed_at: '2025-01-01T00:00:30Z' }))

    expect(preferLocal(newer, older)).toBe(true)
    expect(preferLocal(older, newer)).toBe(false)
  })
})

describe('setActiveRun / beginRun', () => {
  it('clears per-run buffers but never the history list', () => {
    applyServerRun(makeSnapshot([makeStep({ step_id: 's1', status: 'completed' })]))
    workflowExecutionStore.store.setState({ runs: [makeSummary()] })

    setActiveRun('run-9', 'w1')

    const s = getState()
    expect(s.runId).toBe('run-9')
    expect(s.stepStates).toEqual({})
    expect(s.eventLog).toEqual([])
    expect(s.completedStepCount).toBe(0)
    expect(s.runs).toHaveLength(1)
  })

  it('beginRun opens a running overlay for a client-started run', () => {
    beginRun('run-new', 'w1')

    const s = getState()
    expect(s.runId).toBe('run-new')
    expect(s.isRunning).toBe(true)
    expect(s.startedAt).not.toBeNull()
  })
})
