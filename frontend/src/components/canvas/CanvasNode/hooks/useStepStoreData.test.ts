import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'

// ── Mocks ────────────────────────────────────────────────────────────────────

const mockUseStore = vi.hoisted(() => vi.fn())

const mockSelectRoomStepMembers = vi.hoisted(() => vi.fn())
const mockSelectStepIssues = vi.hoisted(() => vi.fn())

vi.mock('@/stores', () => ({
  useStore: mockUseStore,
  workflowStore: {
    store: {},
    selectRoomStepMembers: mockSelectRoomStepMembers,
    selectStepIssues: mockSelectStepIssues,
  },
}))

const { useStepStoreData } = await import('./useStepStoreData')

// ── Tests ────────────────────────────────────────────────────────────────────

describe('useStepStoreData', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns room members and step issues for a step', () => {
    const mockMembers = [{ id: 'm-1', name: 'Alice' }]
    const mockIssues = [{ id: 'i-1', message: 'Missing agent' }]

    let callCount = 0
    mockUseStore.mockImplementation(() => {
      callCount++
      if (callCount === 1) return mockMembers
      return mockIssues
    })

    const { result } = renderHook(() => useStepStoreData('step-1'))

    expect(result.current.roomStepMembers).toBe(mockMembers)
    expect(result.current.stepIssues).toBe(mockIssues)
  })

  it('returns empty arrays when step has no data', () => {
    mockUseStore.mockReturnValue([])

    const { result } = renderHook(() => useStepStoreData('empty-step'))

    expect(result.current.roomStepMembers).toEqual([])
    expect(result.current.stepIssues).toEqual([])
  })

  it('creates selectors with the provided stepId', () => {
    mockUseStore.mockReturnValue([])
    mockSelectRoomStepMembers.mockReturnValue(() => [])
    mockSelectStepIssues.mockReturnValue(() => [])

    renderHook(() => useStepStoreData('my-step-42'))

    expect(mockSelectRoomStepMembers).toHaveBeenCalledWith('my-step-42')
    expect(mockSelectStepIssues).toHaveBeenCalledWith('my-step-42')
  })
})
