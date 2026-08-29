import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@/test/render'
import { useWorkflowRun } from './useWorkflowRun'

const {
  mockSelectActiveWorkflowId,
  mockSelectIsRunning,
  mockSelectRunId,
  mockRunWorkflow,
  mockCancelWorkflow,
  mockBeginRun,
  mockHydrateActive,
} = vi.hoisted(() => ({
  mockSelectActiveWorkflowId: vi.fn<() => string | null>(() => 'wf-001'),
  mockSelectIsRunning: vi.fn<() => boolean>(() => false),
  mockSelectRunId: vi.fn<() => string | null>(() => null),
  mockRunWorkflow: vi.fn(() => Promise.resolve({ execution_id: 'exec-001', workflow_id: 'wf-001', status: 'pending' })),
  mockCancelWorkflow: vi.fn(() => Promise.resolve({ status: 'cancelled' })),
  mockBeginRun: vi.fn(),
  mockHydrateActive: vi.fn(() => Promise.resolve()),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (selector === mockSelectActiveWorkflowId) return mockSelectActiveWorkflowId()
    if (selector === mockSelectIsRunning) return mockSelectIsRunning()
    if (selector === mockSelectRunId) return mockSelectRunId()
    return undefined
  }),
  workflowStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectActiveWorkflowId: mockSelectActiveWorkflowId,
  },
  workflowExecutionStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectIsRunning: mockSelectIsRunning,
    selectRunId: mockSelectRunId,
    beginRun: mockBeginRun,
  },
  workflowLiveStore: {
    hydrateActive: mockHydrateActive,
  },
}))

vi.mock('@/api', () => ({
  api: {
    workflows: {
      run: mockRunWorkflow,
      cancel: mockCancelWorkflow,
    },
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  vi.useFakeTimers({ shouldAdvanceTime: true })
  mockSelectActiveWorkflowId.mockReturnValue('wf-001')
  mockSelectIsRunning.mockReturnValue(false)
  mockSelectRunId.mockReturnValue(null)
  mockRunWorkflow.mockReturnValue(
    Promise.resolve({ execution_id: 'exec-001', workflow_id: 'wf-001', status: 'pending' }),
  )
  mockCancelWorkflow.mockReturnValue(Promise.resolve({ status: 'cancelled' }))
})

describe('useWorkflowRun', () => {
  it('starts in idle status', () => {
    const { result } = renderHook(() => useWorkflowRun('hello'))
    expect(result.current.status).toBe('idle')
    expect(result.current.tooltipText).toBe('Run workflow')
  })

  it('reports running purely from server state, not a local timer', () => {
    // This is what makes the button survive a refresh: it reflects whether the
    // server says a run is in flight, with no client-side countdown.
    mockSelectIsRunning.mockReturnValue(true)
    const { result } = renderHook(() => useWorkflowRun('hello'))

    expect(result.current.status).toBe('running')
    expect(result.current.tooltipText).toBe('Click to cancel')
  })

  it('opens the overlay for the new run so the previous run cannot linger', async () => {
    const { result } = renderHook(() => useWorkflowRun('hello'))

    await act(async () => {
      result.current.handleRun()
      await vi.advanceTimersByTimeAsync(0)
    })

    expect(mockRunWorkflow).toHaveBeenCalledWith('wf-001', { initial_input: 'hello' })
    expect(mockBeginRun).toHaveBeenCalledWith('exec-001', 'wf-001')
    expect(mockHydrateActive).toHaveBeenCalled()
  })

  it('transitions to error on failure', async () => {
    mockRunWorkflow.mockReturnValue(Promise.reject(new Error('fail')))
    const { result } = renderHook(() => useWorkflowRun('hello'))

    await act(async () => {
      result.current.handleRun()
      await vi.advanceTimersByTimeAsync(0)
    })

    expect(result.current.status).toBe('error')
    expect(result.current.tooltipText).toBe('Execution failed to start')
    expect(mockBeginRun).not.toHaveBeenCalled()
  })

  it('sends undefined body when prompt is empty', async () => {
    const { result } = renderHook(() => useWorkflowRun('   '))

    await act(async () => {
      result.current.handleRun()
      await vi.advanceTimersByTimeAsync(0)
    })

    expect(mockRunWorkflow).toHaveBeenCalledWith('wf-001', undefined)
  })

  it('does not fire while a run is already in flight', () => {
    mockSelectIsRunning.mockReturnValue(true)
    const { result } = renderHook(() => useWorkflowRun('hello'))

    act(() => { result.current.handleRun() })

    expect(mockRunWorkflow).not.toHaveBeenCalled()
  })

  it('does not fire when no active workflow', async () => {
    mockSelectActiveWorkflowId.mockReturnValue(null)
    const { result } = renderHook(() => useWorkflowRun('hello'))

    await act(async () => {
      result.current.handleRun()
      await vi.advanceTimersByTimeAsync(0)
    })

    expect(mockRunWorkflow).not.toHaveBeenCalled()
  })

  it('cancels the active run', async () => {
    mockSelectIsRunning.mockReturnValue(true)
    mockSelectRunId.mockReturnValue('exec-001')
    const { result } = renderHook(() => useWorkflowRun('hello'))

    await act(async () => {
      result.current.handleCancel()
      await vi.advanceTimersByTimeAsync(0)
    })

    expect(mockCancelWorkflow).toHaveBeenCalledWith('exec-001')
  })

  it('does not attempt to cancel when there is no active run', async () => {
    const { result } = renderHook(() => useWorkflowRun('hello'))

    await act(async () => {
      result.current.handleCancel()
      await vi.advanceTimersByTimeAsync(0)
    })

    expect(mockCancelWorkflow).not.toHaveBeenCalled()
  })
})
