import { renderHook, act, waitFor } from '@testing-library/react'
import { useStepLastRun } from './useStepLastRun'

const { mockGetStepLastRun, mockSelectActiveWorkflowId } = vi.hoisted(() => ({
  mockGetStepLastRun: vi.fn(),
  mockSelectActiveWorkflowId: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    workflows: { getStepLastRun: mockGetStepLastRun },
  },
}))

vi.mock('@/stores', () => ({
  useStore: (_store: unknown, selector: unknown) => {
    if (selector === mockSelectActiveWorkflowId) return mockSelectActiveWorkflowId() as string | null
    return null
  },
  workflowStore: {
    store: {},
    selectActiveWorkflowId: mockSelectActiveWorkflowId,
  },
}))

describe('useStepLastRun', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSelectActiveWorkflowId.mockReturnValue('wf-1')
  })

  it('fetches last run data on mount', async () => {
    const mockData = {
      execution_id: 'exec-1',
      workflow_execution_id: 'wfexec-1',
      status: 'completed',
      started_at: '2025-01-01T00:00:00Z',
      completed_at: '2025-01-01T00:01:00Z',
      duration_ms: 60000,
      output: 'result text',
      structured_output: null,
      input_tokens: 100,
      output_tokens: 50,
      cost_usd: 0.01,
      phases: [],
    }
    mockGetStepLastRun.mockResolvedValue(mockData)

    const { result } = renderHook(() => useStepLastRun('step-1'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(result.current.error).toBeNull()
    expect(result.current.data).toEqual(mockData)
    expect(mockGetStepLastRun).toHaveBeenCalledWith('wf-1', 'step-1')
  })

  it('sets data to null on 404 error', async () => {
    mockGetStepLastRun.mockRejectedValue(new Error('404 Not Found'))

    const { result } = renderHook(() => useStepLastRun('step-1'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(result.current.error).toBeNull()
    expect(result.current.data).toBeNull()
  })

  it('sets error on non-404 API failure', async () => {
    mockGetStepLastRun.mockRejectedValue(new Error('Server error'))

    const { result } = renderHook(() => useStepLastRun('step-1'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(result.current.error).toBe('Server error')
    expect(result.current.data).toBeNull()
  })

  it('refresh triggers re-fetch', async () => {
    mockGetStepLastRun.mockResolvedValue({
      execution_id: 'exec-1',
      workflow_execution_id: 'wfexec-1',
      status: 'completed',
      started_at: null,
      completed_at: null,
      duration_ms: null,
      output: null,
      structured_output: null,
      input_tokens: null,
      output_tokens: null,
      cost_usd: null,
      phases: null,
    })

    const { result } = renderHook(() => useStepLastRun('step-1'))

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false)
    })

    expect(mockGetStepLastRun).toHaveBeenCalledTimes(1)

    act(() => {
      result.current.refresh()
    })

    await waitFor(() => {
      expect(mockGetStepLastRun).toHaveBeenCalledTimes(2)
    })
  })

  it('does not fetch when workflowId is null', async () => {
    mockSelectActiveWorkflowId.mockReturnValue(null)

    const { result } = renderHook(() => useStepLastRun('step-1'))

    await new Promise((r) => setTimeout(r, 50))

    expect(mockGetStepLastRun).not.toHaveBeenCalled()
    expect(result.current.isLoading).toBe(true)
    expect(result.current.data).toBeNull()
  })
})
