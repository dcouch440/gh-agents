import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@/test/render'
import { useEnterFocusMode } from './useEnterFocusMode'

const { mockEnter, mockGetWorkflowState, mockGetCanvasState } = vi.hoisted(() => ({
  mockEnter: vi.fn(),
  mockGetWorkflowState: vi.fn(),
  mockGetCanvasState: vi.fn(),
}))

vi.mock('@/stores', () => ({
  workflowStore: {
    store: { getState: mockGetWorkflowState },
  },
  canvasStore: {
    store: { getState: mockGetCanvasState },
  },
  focusModeStore: {
    enter: mockEnter,
  },
}))

const { mockTopoSort } = vi.hoisted(() => ({
  mockTopoSort: vi.fn((_steps: unknown[], _edges: unknown[]) => ['s1', 's2', 's3']),
}))

vi.mock('@/utils/topoSort', () => ({
  topoSortStepIds: mockTopoSort,
}))

const makeWorkflowState = () => ({
  steps: { byId: new Map([['s1', { id: 's1' }], ['s2', { id: 's2' }], ['s3', { id: 's3' }]]) },
  edges: { byId: new Map() },
})

beforeEach(() => {
  vi.clearAllMocks()
  mockGetWorkflowState.mockReturnValue(makeWorkflowState())
  mockGetCanvasState.mockReturnValue({ selectedStepIds: new Set<string>() })
})

describe('useEnterFocusMode', () => {
  it('calls focusModeStore.enter with sorted step ids', () => {
    const { result } = renderHook(() => useEnterFocusMode())

    act(() => { result.current() })

    expect(mockEnter).toHaveBeenCalledOnce()
    expect(mockEnter).toHaveBeenCalledWith(['s1', 's2', 's3'], undefined)
  })

  it('respects explicit initialStepId', () => {
    const { result } = renderHook(() => useEnterFocusMode())

    act(() => { result.current('s2') })

    expect(mockEnter).toHaveBeenCalledWith(['s1', 's2', 's3'], 's2')
  })

  it('falls back to first selected step when no explicit id', () => {
    mockGetCanvasState.mockReturnValue({ selectedStepIds: new Set(['s3']) })

    const { result } = renderHook(() => useEnterFocusMode())

    act(() => { result.current() })

    expect(mockEnter).toHaveBeenCalledWith(['s1', 's2', 's3'], 's3')
  })

  it('does not call enter when workflow has no steps', () => {
    mockTopoSort.mockReturnValueOnce([])

    const { result } = renderHook(() => useEnterFocusMode())

    act(() => { result.current() })

    expect(mockEnter).not.toHaveBeenCalled()
  })

  it('returns a stable function reference', () => {
    const { result, rerender } = renderHook(() => useEnterFocusMode())
    const first = result.current
    rerender()
    expect(result.current).toBe(first)
  })
})
