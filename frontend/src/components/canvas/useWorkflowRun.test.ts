import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@/test/render'
import { useWorkflowRun } from './useWorkflowRun'

const { mockSelectActiveWorkflowId, mockRunWorkflow } = vi.hoisted(() => ({
  mockSelectActiveWorkflowId: vi.fn<() => string | null>(() => 'wf-001'),
  mockRunWorkflow: vi.fn(() => Promise.resolve({ id: 'exec-001' })),
}))

vi.mock('@/stores', () => ({
  useStore: vi.fn((_store: unknown, selector: unknown) => {
    if (selector === mockSelectActiveWorkflowId) return mockSelectActiveWorkflowId()
    return undefined
  }),
  workflowStore: {
    store: { getState: vi.fn(), subscribe: vi.fn() },
    selectActiveWorkflowId: mockSelectActiveWorkflowId,
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
  mockRunWorkflow.mockReturnValue(Promise.resolve({ id: 'exec-001' }))
})

describe('useWorkflowRun', () => {
  it('starts in idle status', () => {
    const { result } = renderHook(() => useWorkflowRun('hello'))
    expect(result.current.status).toBe('idle')
    expect(result.current.tooltipText).toBe('Run workflow')
  })

  it('transitions to running then completed', async () => {
    const { result } = renderHook(() => useWorkflowRun('hello'))

    await act(async () => {
      result.current.handleRun()
      await vi.advanceTimersByTimeAsync(0)
    })

    expect(result.current.status).toBe('completed')
    expect(result.current.tooltipText).toBe('Execution started successfully')
    expect(mockRunWorkflow).toHaveBeenCalledWith('wf-001', { initial_input: 'hello' })

    act(() => { vi.advanceTimersByTime(3000) })

    expect(result.current.status).toBe('idle')
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

    act(() => { vi.advanceTimersByTime(3000) })

    expect(result.current.status).toBe('idle')
  })

  it('sends undefined body when prompt is empty', async () => {
    const { result } = renderHook(() => useWorkflowRun('   '))

    await act(async () => {
      result.current.handleRun()
      await vi.advanceTimersByTimeAsync(0)
    })

    expect(mockRunWorkflow).toHaveBeenCalledWith('wf-001', undefined)
  })

  it('does not fire when already running', () => {
    let resolveRun: () => void = () => {}
    mockRunWorkflow.mockReturnValue(new Promise<void>((r) => { resolveRun = r }))

    const { result } = renderHook(() => useWorkflowRun('hello'))

    act(() => { result.current.handleRun() })
    expect(result.current.status).toBe('running')

    // Second call should be no-op
    act(() => { result.current.handleRun() })
    expect(mockRunWorkflow).toHaveBeenCalledOnce()

    act(() => { resolveRun() })
  })

  it('does not fire when no active workflow', async () => {
    mockSelectActiveWorkflowId.mockReturnValue(null)
    const { result } = renderHook(() => useWorkflowRun('hello'))

    await act(async () => {
      result.current.handleRun()
      await vi.advanceTimersByTimeAsync(0)
    })

    expect(mockRunWorkflow).not.toHaveBeenCalled()
    expect(result.current.status).toBe('idle')
  })
})
