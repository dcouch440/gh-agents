import { describe, it, expect, vi, beforeEach } from 'vitest'
import { workflowLiveStore } from '.'
import { hydrateLiveState, hydrateActive } from './hydrate'
import { workflowExecutionStore } from '../workflowExecutionStore'
import { dispatchStore } from '../dispatchStore'
import { agentTraceStore } from '../agentTraceStore'
import type { WorkflowExecutionSummary, WorkflowLiveStateResponse } from '@/types'

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

vi.mock('@/api', () => ({
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

  it('records an error without clearing the workflow id', async () => {
    mockGetLiveState.mockRejectedValue(new Error('network down'))

    await hydrateLiveState('wf-1')

    const s = workflowLiveStore.store.getState()
    expect(s.error).not.toBeNull()
    expect(s.loading).toBe(false)
    expect(s.workflowId).toBe('wf-1')
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
