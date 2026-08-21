import { describe, it, expect, vi, beforeEach } from 'vitest'
import { workflowLiveStore } from '.'
import { viewHistoricalRun, returnToLive } from './history'
import { workflowExecutionStore } from '../workflowExecutionStore'
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

beforeEach(() => {
  vi.clearAllMocks()
  workflowLiveStore.reset()
  workflowExecutionStore.reset()
  agentTraceStore.reset()
  mockGetLiveState.mockResolvedValue(makeLiveState())
  mockGetExecutionTimeline.mockResolvedValue({ entries: [], has_more: false, next_cursor: null })
  mockGetWorkshopStatus.mockRejectedValue(new Error('no workshop'))
})

describe('viewHistoricalRun', () => {
  it('switches workflowExecutionStore into history mode for the selected run', async () => {
    workflowExecutionStore.store.setState({ runs: [makeRun({ id: 'run-old' })] })

    await viewHistoricalRun('run-old')

    const s = workflowExecutionStore.store.getState()
    expect(s.viewMode).toBe('history')
    expect(s.selectedHistoricalRunId).toBe('run-old')
  })

  it('hydrates agentTraceStore with the selected run\'s timeline', async () => {
    mockGetExecutionTimeline.mockResolvedValue({
      entries: [
        {
          id: 'e1', ts: '2025-01-01T00:00:00Z', kind: 'assistant_message',
          step_id: 's-1', step_name: 'Step', agent_name: 'Agent', agent_execution_id: 'ae-1',
          content: 'hi', tool_name: null, tool_call_id: null, input_tokens: 0, output_tokens: 0,
        },
      ],
      has_more: false,
      next_cursor: null,
    })

    await viewHistoricalRun('run-old')

    const s = agentTraceStore.store.getState()
    expect(s.hydratedRunId).toBe('run-old')
    expect(s.order).toEqual(['ae-1'])
    expect(mockGetExecutionTimeline).toHaveBeenCalledWith('run-old', expect.any(Number))
  })

  it('replaces traces from a previously viewed run rather than merging them', async () => {
    agentTraceStore.setHydratedRun('run-a')
    agentTraceStore.store.setState({
      traces: { 'ae-old': { agentExecutionId: 'ae-old', agentName: 'A', stepId: 's-1', events: [] } },
      order: ['ae-old'],
    })

    await viewHistoricalRun('run-b')

    const s = agentTraceStore.store.getState()
    expect(s.hydratedRunId).toBe('run-b')
    expect(s.order).toEqual([])
    expect(s.traces['ae-old']).toBeUndefined()
  })
})

describe('returnToLive', () => {
  it('restores live view mode and re-hydrates from the live run', async () => {
    workflowLiveStore.store.setState({ workflowId: 'wf-1' })
    workflowExecutionStore.store.setState({ viewMode: 'history', selectedHistoricalRunId: 'run-old' })
    mockGetLiveState.mockResolvedValue(makeLiveState({ latest_run: makeRun({ id: 'run-live' }) }))

    await returnToLive()

    const s = workflowExecutionStore.store.getState()
    expect(s.viewMode).toBe('live')
    expect(s.selectedHistoricalRunId).toBeNull()
    expect(mockGetLiveState).toHaveBeenCalledWith('wf-1')
  })

  it('is a no-op call to the live endpoint when no workflow is loaded', async () => {
    await returnToLive()

    expect(mockGetLiveState).not.toHaveBeenCalled()
  })
})
