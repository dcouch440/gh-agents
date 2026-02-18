import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'

// ── Mocks ────────────────────────────────────────────────────────────────────

const mockUseStore = vi.hoisted(() => vi.fn())

const mockSelectStepDocumentDefs = vi.hoisted(() => vi.fn())
const mockSelectRoomStepMembers = vi.hoisted(() => vi.fn())
const mockSelectStepIssues = vi.hoisted(() => vi.fn())

vi.mock('@/stores', () => ({
  useStore: mockUseStore,
  workflowStore: {
    store: {},
    selectStepDocumentDefs: mockSelectStepDocumentDefs,
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

  it('returns document defs, room members, and step issues for a step', () => {
    const mockDocs = [{ id: 'def-1', name: 'README' }]
    const mockMembers = [{ id: 'm-1', name: 'Alice' }]
    const mockIssues = [{ id: 'i-1', message: 'Missing agent' }]

    let callCount = 0
    mockUseStore.mockImplementation(() => {
      callCount++
      if (callCount === 1) return mockDocs
      if (callCount === 2) return mockMembers
      return mockIssues
    })

    const { result } = renderHook(() => useStepStoreData('step-1'))

    expect(result.current.documentDefs).toBe(mockDocs)
    expect(result.current.roomStepMembers).toBe(mockMembers)
    expect(result.current.stepIssues).toBe(mockIssues)
  })

  it('returns empty arrays when step has no data', () => {
    mockUseStore.mockReturnValue([])

    const { result } = renderHook(() => useStepStoreData('empty-step'))

    expect(result.current.documentDefs).toEqual([])
    expect(result.current.roomStepMembers).toEqual([])
    expect(result.current.stepIssues).toEqual([])
  })

  it('creates selectors with the provided stepId', () => {
    mockUseStore.mockReturnValue([])
    mockSelectStepDocumentDefs.mockReturnValue(() => [])
    mockSelectRoomStepMembers.mockReturnValue(() => [])
    mockSelectStepIssues.mockReturnValue(() => [])

    renderHook(() => useStepStoreData('my-step-42'))

    expect(mockSelectStepDocumentDefs).toHaveBeenCalledWith('my-step-42')
    expect(mockSelectRoomStepMembers).toHaveBeenCalledWith('my-step-42')
    expect(mockSelectStepIssues).toHaveBeenCalledWith('my-step-42')
  })
})
