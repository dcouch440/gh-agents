import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@/test/render'
import { useWorkflowRun } from './useWorkflowRun'

const {
  mockSelectActiveWorkflowId,
  mockSelectIsRunning,
  mockRunWorkflow,
  mockBeginRun,
  mockHydrateActive,
} = vi.hoisted(() => ({
  mockSelectActiveWorkflowId: vi.fn<() => string | null>(() => 'wf-001'),
  mockSelectIsRunning: vi.fn<() => boolean>(() => false),
  mockRunWorkflow: vi.fn(() => Promise.resolve({ execution_id: 'exec-001', workflow_id: 'wf-001', status: 'pending' })),
  mockBeginRun: vi.fn(),
  mockHydrateActive: vi.fn(() => Promise.resolve()),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (selector === mockSelectActiveWorkflowId) return mockSelectActiveWorkflowId()
    if (selector === mockSelectIsRunning) return mockSelectIsRunning()
    return undefined
  }),
  workflowStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectActiveWorkflowId: mockSelectActiveWorkflowId,
  },
  workflowExecutionStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectIsRunning: mockSelectIsRunning,
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
    },
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  vi.useFakeTimers({ shouldAdvanceTime: true })
  mockSelectActiveWorkflowId.mockReturnValue('wf-001')
  mockSelectIsRunning.mockReturnValue(false)
  mockRunWorkflow.mockReturnValue(
    Promise.resolve({ execution_id: 'exec-001', workflow_id: 'wf-001', status: 'pending' }),
  )
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
    expect(result.current.tooltipText).toBe('Workflow is running...')
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
})
