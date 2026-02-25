import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'

// ── Mocks ────────────────────────────────────────────────────────────────────

const mockUseStore = vi.hoisted(() => vi.fn())

const mockSelectRoomStepMembers = vi.hoisted(() => vi.fn())

vi.mock('@/stores', () => ({
  useStore: mockUseStore,
  workflowStore: {
    store: {},
    selectRoomStepMembers: mockSelectRoomStepMembers,
  },
}))

const { useStepStoreData } = await import('./useStepStoreData')

// ── Tests ────────────────────────────────────────────────────────────────────

describe('useStepStoreData', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns room members for a step', () => {
    const mockMembers = [{ id: 'm-1', name: 'Alice' }]

    mockUseStore.mockReturnValue(mockMembers)

    const { result } = renderHook(() => useStepStoreData('step-1'))

    expect(result.current.roomStepMembers).toBe(mockMembers)
  })

  it('returns empty arrays when step has no data', () => {
    mockUseStore.mockReturnValue([])

    const { result } = renderHook(() => useStepStoreData('empty-step'))

    expect(result.current.roomStepMembers).toEqual([])
  })

  it('creates selectors with the provided stepId', () => {
    mockUseStore.mockReturnValue([])
    mockSelectRoomStepMembers.mockReturnValue(() => [])

    renderHook(() => useStepStoreData('my-step-42'))

    expect(mockSelectRoomStepMembers).toHaveBeenCalledWith('my-step-42')
  })
})
