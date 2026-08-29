import { resolveNodeStatus } from './resolveNodeStatus'
import type { BaselineStepState, LiveDispatch } from '@/stores/workflowLiveStore'
import type { StepExecutionState, StepExecutionStatus } from '@/stores/workflowExecutionStore'

const makeBaseline = (overrides: Partial<BaselineStepState> = {}): BaselineStepState => ({
  stepId: 's1',
  name: 'Scanner',
  executionMode: 'workforce',
  baselineStatus: 'idle',
  pinned: false,
  hasRunSummary: false,
  isRunningInActiveRun: false,
  ...overrides,
})

const makeRunState = (status: StepExecutionStatus): StepExecutionState => ({
  status,
  stepName: 'Scanner',
  agentId: null,
  executionId: 'exec-1',
  output: null,
  error: null,
  inputTokens: null,
  outputTokens: null,
  durationMs: null,
  forEachProgress: null,
  startedAt: null,
  completedAt: null,
})

const makeDispatch = (status: string): LiveDispatch => ({
  stepId: 's1',
  executionId: 'd1',
  status,
  instruction: 'Add a researcher',
  createdAt: '2025-01-01T00:00:00Z',
  result: null,
  traceLen: 0,
  source: 'registry',
})

describe('resolveNodeStatus', () => {
  it('shows a design spinner while a dispatch is running', () => {
    const r = resolveNodeStatus({
      baseline: makeBaseline(),
      runState: undefined,
      dispatch: makeDispatch('running'),
    })

    expect(r.designStatus).toBe('running')
    expect(r.status).toBe('idle')
  })

  it('lets a running generation outrank a pinned baseline', () => {
    const r = resolveNodeStatus({
      baseline: makeBaseline({ pinned: true }),
      runState: undefined,
      dispatch: makeDispatch('running'),
    })

    expect(r.designStatus).toBe('running')
    expect(r.pinned).toBe(true)
  })

  it('shows the run overlay while the current run executes this step', () => {
    const r = resolveNodeStatus({
      baseline: makeBaseline(),
      runState: makeRunState('running'),
      dispatch: null,
    })

    expect(r.status).toBe('running')
    expect(r.designStatus).toBeNull()
  })

  it('shows the run result once the step finishes', () => {
    const r = resolveNodeStatus({
      baseline: makeBaseline(),
      runState: makeRunState('error'),
      dispatch: null,
    })

    expect(r.status).toBe('error')
  })

  // ── The pinned guarantee ──────────────────────────────────────────────────

  it('keeps a pinned node completed when a new run has not reached it', () => {
    // The user's case: start a new run, and a pinned node elsewhere on the board
    // must not blank out just because the run overlay was swapped.
    const r = resolveNodeStatus({
      baseline: makeBaseline({ pinned: true, baselineStatus: 'completed' }),
      runState: undefined,
      dispatch: null,
    })

    expect(r.status).toBe('success')
    expect(r.pinned).toBe(true)
  })

  it('keeps a pinned node completed when the run marks it merely pending', () => {
    const r = resolveNodeStatus({
      baseline: makeBaseline({ pinned: true, baselineStatus: 'completed' }),
      runState: makeRunState('pending'),
      dispatch: null,
    })

    expect(r.status).toBe('success')
  })

  it('lets the new run take over a pinned node once it actually runs it', () => {
    const r = resolveNodeStatus({
      baseline: makeBaseline({ pinned: true, baselineStatus: 'completed' }),
      runState: makeRunState('running'),
      dispatch: null,
    })

    expect(r.status).toBe('running')
    expect(r.pinned).toBe(true)
  })

  it('treats a stored run summary like a pin', () => {
    const r = resolveNodeStatus({
      baseline: makeBaseline({ hasRunSummary: true, baselineStatus: 'completed' }),
      runState: undefined,
      dispatch: null,
    })

    expect(r.status).toBe('success')
    expect(r.pinned).toBe(false)
  })

  // ── Remaining rules ───────────────────────────────────────────────────────

  it('reports a failed generation as an error', () => {
    const r = resolveNodeStatus({
      baseline: makeBaseline({ baselineStatus: 'error' }),
      runState: undefined,
      dispatch: makeDispatch('failed'),
    })

    expect(r.status).toBe('error')
    expect(r.designStatus).toBe('failed')
  })

  it('shows a steady design marker for a configured node', () => {
    const r = resolveNodeStatus({
      baseline: makeBaseline({ baselineStatus: 'configured' }),
      runState: undefined,
      dispatch: makeDispatch('completed'),
    })

    expect(r.status).toBe('idle')
    expect(r.designStatus).toBe('completed')
  })

  it('falls back to idle for an untouched node', () => {
    const r = resolveNodeStatus({ baseline: makeBaseline(), runState: undefined, dispatch: null })

    expect(r.status).toBe('idle')
    expect(r.designStatus).toBeNull()
    expect(r.pinned).toBe(false)
  })

  it('survives a missing baseline (step added since the last fetch)', () => {
    const r = resolveNodeStatus({
      baseline: null,
      runState: makeRunState('running'),
      dispatch: null,
    })

    expect(r.status).toBe('running')
    expect(r.pinned).toBe(false)
  })
})
