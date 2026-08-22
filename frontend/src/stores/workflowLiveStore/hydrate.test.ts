import { describe, it, expect, vi, beforeEach } from 'vitest'
import { workflowLiveStore } from '.'
import { ApiError } from '@/api'
import { hydrateLiveState, hydrateActive, UNCONFIRMED_LIMIT, DEFAULT_THROTTLE_MS } from './hydrate'
import { workflowExecutionStore } from '../workflowExecutionStore'
import { dispatchStore } from '../dispatchStore'
import { agentTraceStore } from '../agentTraceStore'
import type { WorkflowExecutionSummary, WorkflowLiveStateResponse } from '@/types'
import type * as ApiModule from '@/api'

const {
  mockGetLiveState,
  mockDispatchTrace,
  mockGetStepDispatchHistory,
  mockGetExecutionTimeline,
  mockGetWorkshopStatus,
} = vi.hoisted(() => ({
  mockGetLiveState: vi.fn(),
  mockDispatchTrace: vi.fn(),
  mockGetStepDispatchHistory: vi.fn(),
  mockGetExecutionTimeline: vi.fn(() => Promise.resolve({ entries: [], has_more: false, next_cursor: null })),
  mockGetWorkshopStatus: vi.fn(() => Promise.reject(new Error('no workshop'))),
}))

// Only `api` is stubbed — the error guards stay real so the throttling tests
// exercise the same narrowing the app does.
vi.mock('@/api', async (importOriginal) => ({
  ...(await importOriginal<typeof ApiModule>()),
  api: {
    workflows: {
      getLiveState: mockGetLiveState,
      getStepDispatchHistory: mockGetStepDispatchHistory,
      getExecutionTimeline: mockGetExecutionTimeline,
      getWorkshopStatus: mockGetWorkshopStatus,
    },
    dispatch: {
      trace: mockDispatchTrace,
    },
  },
}))

const makeRun = (overrides: Partial<WorkflowExecutionSummary> = {}): WorkflowExecutionSummary => ({
  id: 'run-1',
  workflow_id: 'wf-1',
  status: 'completed',
  started_at: '2025-01-01T00:00:00Z',
  completed_at: '2025-01-01T00:01:00Z',
  outputs: null,
  error: null,
  execution_mode: 'single',
  template_id: null,
  ...overrides,
})

const makeLiveState = (
  overrides: Partial<WorkflowLiveStateResponse> = {},
): WorkflowLiveStateResponse => ({
  workflow_id: 'wf-1',
  server_time: '2025-01-01T00:02:00Z',
  active_run: null,
  latest_run: null,
  run_steps: [],
  steps: [],
  dispatches: [],
  generating: false,
  ...overrides,
})

const makeTrace = (executionId: string, stepId: string) => ({
  execution_id: executionId,
  step_id: stepId,
  workflow_id: 'wf-1',
  status: 'completed',
  instruction: 'Configure',
  trace: [],
  result: null,
})

beforeEach(() => {
  vi.clearAllMocks()
  workflowLiveStore.reset()
  workflowExecutionStore.reset()
  agentTraceStore.reset()
  dispatchStore.store.setState({ byStep: {} })
  mockGetLiveState.mockResolvedValue(makeLiveState())
  mockDispatchTrace.mockImplementation((id: string) => Promise.resolve(makeTrace(id, 'step-1')))
  mockGetStepDispatchHistory.mockImplementation((_wf: string, stepId: string) =>
    Promise.resolve(makeTrace('ae-1', stepId)),
  )
  mockGetExecutionTimeline.mockResolvedValue({ entries: [], has_more: false, next_cursor: null })
  mockGetWorkshopStatus.mockRejectedValue(new Error('no workshop'))
})

describe('hydrateLiveState', () => {
  it('stores the baseline layer for every step', async () => {
    mockGetLiveState.mockResolvedValue(makeLiveState({
      steps: [
        { step_id: 's1', name: 'Scanner', execution_mode: 'workforce', baseline_status: 'completed', pinned: true, has_run_summary: false, is_running_in_active_run: false },
      ],
    }))

    await hydrateLiveState('wf-1')

    const baseline = workflowLiveStore.store.getState().baselineByStep['s1']
    expect(baseline?.pinned).toBe(true)
    expect(baseline?.baselineStatus).toBe('completed')
  })

  it('prefers the active run over the latest finished one', async () => {
    mockGetLiveState.mockResolvedValue(makeLiveState({
      active_run: makeRun({ id: 'run-active', status: 'running', completed_at: null }),
      latest_run: makeRun({ id: 'run-active', status: 'running', completed_at: null }),
    }))

    await hydrateLiveState('wf-1')

    expect(workflowExecutionStore.store.getState().runId).toBe('run-active')
    expect(workflowExecutionStore.store.getState().isRunning).toBe(true)
  })

  it('preserves the server dispatch ordering verbatim', async () => {
    // Regression guard for the old hook reading tasks[length - 1] against a
    // newest-first list, which hydrated the oldest dispatch.
    mockGetLiveState.mockResolvedValue(makeLiveState({
      dispatches: [
        { step_id: 's2', execution_id: 'newest', status: 'running', instruction: 'b', created_at: '2025-01-01T00:05:00Z', result: null, trace_len: 0, source: 'registry' },
        { step_id: 's1', execution_id: 'older', status: 'completed', instruction: 'a', created_at: '2025-01-01T00:01:00Z', result: null, trace_len: 0, source: 'registry' },
      ],
    }))

    await hydrateLiveState('wf-1')

    const ids = workflowLiveStore.store.getState().dispatches.map((d) => d.executionId)
    expect(ids).toEqual(['newest', 'older'])
  })

  it('routes a registry dispatch to the dispatch trace endpoint', async () => {
    mockGetLiveState.mockResolvedValue(makeLiveState({
      dispatches: [
        { step_id: 's1', execution_id: 'exec-1', status: 'running', instruction: 'a', created_at: '2025-01-01T00:01:00Z', result: null, trace_len: 0, source: 'registry' },
      ],
    }))

    await hydrateLiveState('wf-1')

    expect(mockDispatchTrace).toHaveBeenCalledWith('exec-1')
    expect(mockGetStepDispatchHistory).not.toHaveBeenCalled()
  })

  it('routes a persisted dispatch to the step history endpoint', async () => {
    // After a server restart the in-memory registry is empty and the execution
    // id is an agent_execution id the dispatch route cannot resolve.
    mockGetLiveState.mockResolvedValue(makeLiveState({
      dispatches: [
        { step_id: 's1', execution_id: 'ae-1', status: 'completed', instruction: 'a', created_at: '2025-01-01T00:01:00Z', result: null, trace_len: 3, source: 'persisted' },
      ],
    }))

    await hydrateLiveState('wf-1')

    expect(mockGetStepDispatchHistory).toHaveBeenCalledWith('wf-1', 's1')
    expect(mockDispatchTrace).not.toHaveBeenCalled()
  })

  it('derives isGenerating from the server', async () => {
    mockGetLiveState.mockResolvedValue(makeLiveState({ generating: true }))

    await hydrateLiveState('wf-1')

    expect(workflowLiveStore.store.getState().isGenerating).toBe(true)
  })

  it('prunes view entries for steps the server no longer reports', async () => {
    dispatchStore.store.setState({
      byStep: {
        stale: { executionId: 'x', stepId: 'stale', status: 'completed', instruction: '', message: null, summary: null, error: null, startedAt: '', trace: [], tokenBuffer: '' },
      },
    })

    await hydrateLiveState('wf-1')

    expect(dispatchStore.store.getState().byStep['stale']).toBeUndefined()
  })

  it('falls back to workshop state only when there has never been a run', async () => {
    await hydrateLiveState('wf-1')

    expect(mockGetWorkshopStatus).toHaveBeenCalledWith('wf-1')
  })

  it('does not reach for workshop state when a run exists', async () => {
    mockGetLiveState.mockResolvedValue(makeLiveState({ latest_run: makeRun() }))

    await hydrateLiveState('wf-1')

    expect(mockGetWorkshopStatus).not.toHaveBeenCalled()
  })

  it('drops an unconfirmable generating spinner rather than leaving it stuck', async () => {
    // handleGenerate sets isGenerating optimistically and relies on the next
    // hydrate for server truth. If the server is unreachable — a stale build, a
    // network blip — the spinner must not run forever.
    workflowLiveStore.store.setState({ isGenerating: true })
    mockGetLiveState.mockRejectedValue(new Error('404 Not Found'))

    await hydrateLiveState('wf-1')
    expect(workflowLiveStore.store.getState().isGenerating).toBe(true)

    await hydrateLiveState('wf-1')
    expect(workflowLiveStore.store.getState().isGenerating).toBe(false)
  })

  it('restores the spinner as soon as the server confirms it again', async () => {
    workflowLiveStore.store.setState({ isGenerating: true, consecutiveFailures: 5 })
    mockGetLiveState.mockResolvedValue(makeLiveState({ generating: true }))

    await hydrateLiveState('wf-1')

    const s = workflowLiveStore.store.getState()
    expect(s.isGenerating).toBe(true)
    expect(s.consecutiveFailures).toBe(0)
  })

  // ── Rate limiting ─────────────────────────────────────────────────────────

  const rateLimited = (retryAfterMs: number | null) =>
    ApiError.rateLimit('/api/workflows/wf-1/live-state', 'Too Many Requests', null, retryAfterMs)

  it('treats a 429 as backpressure, not a failure', async () => {
    // Being throttled says nothing about the workflow. The view we already have
    // is still correct, so it must survive — and the failure budget, which
    // exists for "the server is broken", must not be spent on it.
    mockGetLiveState.mockResolvedValueOnce(makeLiveState({ generating: true }))
    await hydrateLiveState('wf-1')

    mockGetLiveState.mockRejectedValue(rateLimited(4000))
    await hydrateLiveState('wf-1')

    const s = workflowLiveStore.store.getState()
    expect(s.consecutiveFailures).toBe(0)
    expect(s.error).toBeNull()
    expect(s.isGenerating).toBe(true)
    expect(s.throttledUntilMs).not.toBeNull()
  })

  it('honours the wait the server asked for', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2025-01-01T00:00:00Z'))
    try {
      mockGetLiveState.mockRejectedValue(rateLimited(4000))

      await hydrateLiveState('wf-1')

      expect(workflowLiveStore.store.getState().throttledUntilMs).toBe(Date.now() + 4000)
    } finally {
      vi.useRealTimers()
    }
  })

  it('falls back to a default wait when the server does not say', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2025-01-01T00:00:00Z'))
    try {
      mockGetLiveState.mockRejectedValue(rateLimited(null))

      await hydrateLiveState('wf-1')

      expect(workflowLiveStore.store.getState().throttledUntilMs).toBe(
        Date.now() + DEFAULT_THROTTLE_MS,
      )
    } finally {
      vi.useRealTimers()
    }
  })

  it('clears the throttle once a request gets through', async () => {
    mockGetLiveState.mockRejectedValueOnce(rateLimited(4000))
    await hydrateLiveState('wf-1')
    expect(workflowLiveStore.store.getState().throttledUntilMs).not.toBeNull()

    mockGetLiveState.mockResolvedValue(makeLiveState())
    await hydrateLiveState('wf-1')

    expect(workflowLiveStore.store.getState().throttledUntilMs).toBeNull()
  })

  it('records a throttle hit from a trace fetch instead of swallowing it', async () => {
    // These used to be swallowed wholesale, which rendered as an empty panel
    // with nothing to explain it.
    mockGetLiveState.mockResolvedValue(makeLiveState({
      dispatches: [
        { step_id: 's1', execution_id: 'exec-1', status: 'running', instruction: 'a', created_at: '2025-01-01T00:01:00Z', result: null, trace_len: 0, source: 'registry' },
      ],
    }))
    mockDispatchTrace.mockRejectedValue(rateLimited(3000))

    await hydrateLiveState('wf-1')

    expect(workflowLiveStore.store.getState().throttledUntilMs).not.toBeNull()
  })

  // ── Optimistic generate ───────────────────────────────────────────────────

  it('holds an optimistic spinner while the server has not caught up', async () => {
    // POST /generate spawns its pipeline and returns before registering
    // anything, so the first read back honestly says "not generating".
    workflowLiveStore.setGenerating(true)
    mockGetLiveState.mockResolvedValue(makeLiveState({ generating: false }))

    for (let i = 0; i < UNCONFIRMED_LIMIT; i++) {
      await hydrateLiveState('wf-1')
      expect(workflowLiveStore.store.getState().isGenerating).toBe(true)
    }

    await hydrateLiveState('wf-1')
    expect(workflowLiveStore.store.getState().isGenerating).toBe(false)
  })

  it('settles immediately once the server confirms the generate', async () => {
    workflowLiveStore.setGenerating(true)
    mockGetLiveState.mockResolvedValue(makeLiveState({ generating: true }))

    await hydrateLiveState('wf-1')

    const s = workflowLiveStore.store.getState()
    expect(s.isGenerating).toBe(true)
    expect(s.unconfirmedGenerating).toBe(0)
  })

  it('does not invent a spinner the server never reported', async () => {
    mockGetLiveState.mockResolvedValue(makeLiveState({ generating: false }))

    await hydrateLiveState('wf-1')

    expect(workflowLiveStore.store.getState().isGenerating).toBe(false)
  })

  // ── Timeline ──────────────────────────────────────────────────────────────

  it('keeps asking the timeline every tick, since debug WS events are opt-in and a run with nothing yet can produce something on the next poll', async () => {
    mockGetLiveState.mockResolvedValue(makeLiveState({ latest_run: makeRun() }))
    mockGetExecutionTimeline.mockResolvedValue({ entries: [], has_more: false, next_cursor: null })

    await hydrateLiveState('wf-1')
    await hydrateLiveState('wf-1')
    await hydrateLiveState('wf-1')

    expect(mockGetExecutionTimeline).toHaveBeenCalledTimes(3)
  })

  it('asks again for a different run', async () => {
    mockGetExecutionTimeline.mockResolvedValue({ entries: [], has_more: false, next_cursor: null })

    mockGetLiveState.mockResolvedValue(makeLiveState({ latest_run: makeRun({ id: 'run-1' }) }))
    await hydrateLiveState('wf-1')

    mockGetLiveState.mockResolvedValue(makeLiveState({ latest_run: makeRun({ id: 'run-2' }) }))
    await hydrateLiveState('wf-1')

    expect(mockGetExecutionTimeline).toHaveBeenCalledTimes(2)
  })

  it('records a throttled timeline so the poll backs off', async () => {
    mockGetLiveState.mockResolvedValue(makeLiveState({ latest_run: makeRun() }))
    mockGetExecutionTimeline.mockRejectedValue(rateLimited(5000))

    await hydrateLiveState('wf-1')

    expect(workflowLiveStore.store.getState().throttledUntilMs).not.toBeNull()
  })

  it('records an error without clearing the workflow id', async () => {
    mockGetLiveState.mockRejectedValue(new Error('network down'))

    await hydrateLiveState('wf-1')

    const s = workflowLiveStore.store.getState()
    expect(s.error).not.toBeNull()
    expect(s.loading).toBe(false)
    expect(s.workflowId).toBe('wf-1')
  })

  // ── Agent trace wiring ───────────────────────────────────────────────────

  it('does not re-stamp an already-hydrated run, and repeated polling does not lose its traces', async () => {
    // Regression for the "started resets hydratedRunId to null" race: once
    // `agentTraceStore` correctly reflects the run on screen, `setHydratedRun`
    // must be a no-op. The timeline is still re-polled every tick (debug WS
    // events are opt-in), but the richer-wins merge must not lose what is
    // already there.
    mockGetLiveState.mockResolvedValue(makeLiveState({ latest_run: makeRun({ id: 'run-1' }) }))
    agentTraceStore.setHydratedRun('run-1')
    agentTraceStore.store.setState({
      traces: { 'ae-1': { agentExecutionId: 'ae-1', agentName: 'A', stepId: 's-1', events: [] } },
      order: ['ae-1'],
    })

    await hydrateLiveState('wf-1')

    const s = agentTraceStore.store.getState()
    expect(s.hydratedRunId).toBe('run-1')
    expect(s.order).toEqual(['ae-1'])
    expect(s.traces['ae-1']).toBeDefined()
    expect(mockGetExecutionTimeline).toHaveBeenCalledWith('run-1', expect.any(Number))
  })

  it('grows a trace across ticks from repeated polling alone, with no WS involved', async () => {
    mockGetLiveState.mockResolvedValue(makeLiveState({ latest_run: makeRun({ id: 'run-1', status: 'running' }) }))
    mockGetExecutionTimeline.mockResolvedValueOnce({
      entries: [
        { id: 'e1', ts: '2025-01-01T00:00:00Z', kind: 'system_prompt', step_id: 's-1', step_name: 'Step', agent_name: 'A', agent_execution_id: 'ae-1', content: 'sys', tool_name: null, tool_call_id: null, input_tokens: 0, output_tokens: 0 },
      ],
      has_more: false,
      next_cursor: null,
    })
    await hydrateLiveState('wf-1')
    expect(agentTraceStore.store.getState().traces['ae-1']?.events).toHaveLength(1)

    mockGetExecutionTimeline.mockResolvedValueOnce({
      entries: [
        { id: 'e1', ts: '2025-01-01T00:00:00Z', kind: 'system_prompt', step_id: 's-1', step_name: 'Step', agent_name: 'A', agent_execution_id: 'ae-1', content: 'sys', tool_name: null, tool_call_id: null, input_tokens: 0, output_tokens: 0 },
        { id: 'e2', ts: '2025-01-01T00:00:01Z', kind: 'user_message', step_id: 's-1', step_name: 'Step', agent_name: 'A', agent_execution_id: 'ae-1', content: 'go', tool_name: null, tool_call_id: null, input_tokens: 0, output_tokens: 0 },
      ],
      has_more: false,
      next_cursor: null,
    })
    await hydrateLiveState('wf-1')

    expect(agentTraceStore.store.getState().traces['ae-1']?.events).toHaveLength(2)
  })

  it('does not touch agentTraceStore while the user is viewing history', async () => {
    mockGetLiveState.mockResolvedValue(makeLiveState({ latest_run: makeRun({ id: 'run-live' }) }))
    workflowExecutionStore.store.setState({ viewMode: 'history' })
    agentTraceStore.setHydratedRun('run-historical')

    await hydrateLiveState('wf-1')

    const s = agentTraceStore.store.getState()
    expect(s.hydratedRunId).toBe('run-historical')
    expect(mockGetExecutionTimeline).not.toHaveBeenCalled()
  })
})

describe('hydrateActive', () => {
  it('is a no-op when no workflow is loaded', async () => {
    await hydrateActive()
    expect(mockGetLiveState).not.toHaveBeenCalled()
  })

  it('re-hydrates whichever workflow is loaded', async () => {
    await hydrateLiveState('wf-1')
    mockGetLiveState.mockClear()

    await hydrateActive()

    expect(mockGetLiveState).toHaveBeenCalledWith('wf-1')
  })
})
