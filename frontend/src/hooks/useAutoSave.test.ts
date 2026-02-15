import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'

const mockSaveAllDirtySteps = vi.fn(() => Promise.resolve())

let storeState = { dirty: false, dirtyStepIds: new Set<string>() }

vi.mock('@/stores', () => {
  return {
    useStore: (_store: unknown, selector: (s: typeof storeState) => unknown) => selector(storeState),
    workflowStore: {
      store: {},
      selectDirty: (s: typeof storeState) => s.dirty,
      selectDirtyStepIds: (s: typeof storeState) => s.dirtyStepIds,
      saveAllDirtySteps: () => mockSaveAllDirtySteps(),
    },
  }
})

vi.mock('@/constants', () => ({
  AUTO_SAVE_DEBOUNCE_MS: 500,
}))

import { useAutoSave } from './useAutoSave'

beforeEach(() => {
  vi.useFakeTimers()
  storeState = { dirty: false, dirtyStepIds: new Set() }
  mockSaveAllDirtySteps.mockClear()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('useAutoSave', () => {
  it('saves after debounce when dirty becomes true', () => {
    const { rerender } = renderHook(({ dirty }) => {
      storeState = { dirty, dirtyStepIds: dirty ? new Set(['step-1']) : new Set() }
      return useAutoSave(true)
    }, { initialProps: { dirty: false } })

    // Become dirty
    rerender({ dirty: true })

    expect(mockSaveAllDirtySteps).not.toHaveBeenCalled()

    act(() => {
      vi.advanceTimersByTime(500)
    })

    expect(mockSaveAllDirtySteps).toHaveBeenCalledOnce()
  })

  it('does not save when disabled', () => {
    const { rerender } = renderHook(({ dirty }) => {
      storeState = { dirty, dirtyStepIds: dirty ? new Set(['step-1']) : new Set() }
      return useAutoSave(false)
    }, { initialProps: { dirty: false } })

    rerender({ dirty: true })

    act(() => {
      vi.advanceTimersByTime(500)
    })

    expect(mockSaveAllDirtySteps).not.toHaveBeenCalled()
  })

  it('flush() forces immediate save', () => {
    const { result, rerender } = renderHook(({ dirty }) => {
      storeState = { dirty, dirtyStepIds: dirty ? new Set(['step-1']) : new Set() }
      return useAutoSave(true)
    }, { initialProps: { dirty: false } })

    rerender({ dirty: true })

    act(() => {
      result.current.flush()
    })

    expect(mockSaveAllDirtySteps).toHaveBeenCalledOnce()
  })

  it('flushes pending save on unmount', () => {
    const { rerender, unmount } = renderHook(({ dirty }) => {
      storeState = { dirty, dirtyStepIds: dirty ? new Set(['step-1']) : new Set() }
      return useAutoSave(true)
    }, { initialProps: { dirty: false } })

    rerender({ dirty: true })
    unmount()

    expect(mockSaveAllDirtySteps).toHaveBeenCalledOnce()
  })

  it('resets debounce when dirty changes again', () => {
    const { rerender } = renderHook(({ dirty }) => {
      storeState = { dirty, dirtyStepIds: dirty ? new Set(['step-1']) : new Set() }
      return useAutoSave(true)
    }, { initialProps: { dirty: false } })

    // First dirty
    rerender({ dirty: true })
    act(() => {
      vi.advanceTimersByTime(300)
    })

    // Clean then dirty again — simulates new edit
    rerender({ dirty: false })
    rerender({ dirty: true })
    act(() => {
      vi.advanceTimersByTime(300)
    })

    // Should not have fired yet (only 300ms since last trigger)
    expect(mockSaveAllDirtySteps).not.toHaveBeenCalled()

    act(() => {
      vi.advanceTimersByTime(200)
    })

    expect(mockSaveAllDirtySteps).toHaveBeenCalledOnce()
  })
})
