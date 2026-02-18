import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'

// ── Mocks ────────────────────────────────────────────────────────────────────

const mockUseStore = vi.hoisted(() => vi.fn())

vi.mock('@/stores', () => ({
  useStore: mockUseStore,
  workflowExecutionStore: {
    store: {},
    selectStepState: (id: string) => `step-selector-${id}`,
  },
  stepStreamStore: {
    store: {},
  },
}))

vi.mock('../execution', () => ({
  toExecutionStatus: (status: string | undefined) => {
    if (!status || status === 'idle') return 'idle'
    if (status === 'success') return 'completed'
    if (status === 'error') return 'failed'
    return status
  },
}))

const { useDynamicNodeExecution } = await import('./useDynamicNodeExecution')

// ── Tests ────────────────────────────────────────────────────────────────────

describe('useDynamicNodeExecution', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns idle state when no execution data exists', () => {
    // First call: workflowExecutionStore → null (no step state)
    // Second call: stepStreamStore → 'idle'
    mockUseStore.mockReturnValueOnce(null).mockReturnValueOnce('idle')

    const { result } = renderHook(() =>
      useDynamicNodeExecution('step-1', false, null),
    )

    expect(result.current.isExecuting).toBe(false)
    expect(result.current.stepExecStatus).toBe('idle')
    expect(result.current.resolvedExecStatus).toBe('idle')
  })

  it('reads from workflowExecutionStore for non-agent nodes', () => {
    // First call: workflowExecutionStore → running step
    // Second call: stepStreamStore → 'idle' (non-agent)
    mockUseStore
      .mockReturnValueOnce({ status: 'success' })
      .mockReturnValueOnce('idle')

    const { result } = renderHook(() =>
      useDynamicNodeExecution('step-1', false, null),
    )

    expect(result.current.stepExecStatus).toBe('completed')
    expect(result.current.resolvedExecStatus).toBe('completed')
    expect(result.current.isExecuting).toBe(true)
  })

  it('reads from stepStreamStore for agent nodes', () => {
    // First call: workflowExecutionStore → null
    // Second call: stepStreamStore → 'running' (agent source)
    mockUseStore
      .mockReturnValueOnce(null)
      .mockReturnValueOnce('running')

    const { result } = renderHook(() =>
      useDynamicNodeExecution('step-1', true, 'agent-1'),
    )

    expect(result.current.agentSourceStatus).toBe('running')
    expect(result.current.isExecuting).toBe(true)
    expect(result.current.resolvedExecStatus).toBe('running')
  })

  it('handles missing rosterAgentId gracefully for agent nodes', () => {
    // First call: workflowExecutionStore → null
    // Second call: stepStreamStore → 'idle' (no matching agent)
    mockUseStore
      .mockReturnValueOnce(null)
      .mockReturnValueOnce('idle')

    const { result } = renderHook(() =>
      useDynamicNodeExecution('step-1', true, null),
    )

    expect(result.current.isExecuting).toBe(false)
    expect(result.current.agentSourceStatus).toBe('idle')
  })
})
