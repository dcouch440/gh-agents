import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { canvasStore } from '@/stores'
import { useGroupHoverDelay } from './useGroupHoverDelay'
import { CANVAS } from './constants'

const makeMouseEvent = () => ({}) as React.MouseEvent

const makeProtocolNode = (id: string) => ({
  id,
  data: { isProtocol: true } as Record<string, unknown>,
})

const makeMemberNode = (id: string) => ({
  id,
  data: { isProtocol: false } as Record<string, unknown>,
})

describe('useGroupHoverDelay', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    canvasStore.reset()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('immediate self-hover', () => {
    it('sets hoveredStepId immediately on protocol node enter', () => {
      const { result } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeProtocolNode('proto-1'))
      })

      expect(canvasStore.store.getState().hoveredStepId).toBe('proto-1')
    })

    it('sets hoveredStepId immediately on member node enter', () => {
      const { result } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeMemberNode('doc-1'))
      })

      expect(canvasStore.store.getState().hoveredStepId).toBe('doc-1')
    })

    it('does not set hoveredProtocolId immediately on protocol node enter', () => {
      const { result } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeProtocolNode('proto-1'))
      })

      expect(canvasStore.store.getState().hoveredProtocolId).toBeNull()
    })
  })

  describe('delayed group hover', () => {
    it('sets hoveredProtocolId after GROUP_HOVER_DELAY_MS', () => {
      const { result } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeProtocolNode('proto-1'))
      })

      act(() => {
        vi.advanceTimersByTime(CANVAS.GROUP_HOVER_DELAY_MS)
      })

      expect(canvasStore.store.getState().hoveredProtocolId).toBe('proto-1')
    })

    it('does not set hoveredProtocolId before delay elapses', () => {
      const { result } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeProtocolNode('proto-1'))
      })

      act(() => {
        vi.advanceTimersByTime(CANVAS.GROUP_HOVER_DELAY_MS - 1)
      })

      expect(canvasStore.store.getState().hoveredProtocolId).toBeNull()
    })

    it('does not start a timer for non-protocol nodes', () => {
      const { result } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeMemberNode('doc-1'))
      })

      act(() => {
        vi.advanceTimersByTime(CANVAS.GROUP_HOVER_DELAY_MS)
      })

      expect(canvasStore.store.getState().hoveredStepId).toBe('doc-1')
      expect(canvasStore.store.getState().hoveredProtocolId).toBeNull()
    })
  })

  describe('cancellation on mouse leave', () => {
    it('cancels pending group hover when mouse leaves before delay', () => {
      const { result } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeProtocolNode('proto-1'))
      })

      act(() => {
        vi.advanceTimersByTime(100)
        result.current.onNodeMouseLeave()
      })

      act(() => {
        vi.advanceTimersByTime(CANVAS.GROUP_HOVER_DELAY_MS)
      })

      expect(canvasStore.store.getState().hoveredStepId).toBeNull()
      expect(canvasStore.store.getState().hoveredProtocolId).toBeNull()
    })

    it('clears both fields on mouse leave after group hover already fired', () => {
      const { result } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeProtocolNode('proto-1'))
      })

      act(() => {
        vi.advanceTimersByTime(CANVAS.GROUP_HOVER_DELAY_MS)
      })

      expect(canvasStore.store.getState().hoveredProtocolId).toBe('proto-1')

      act(() => {
        result.current.onNodeMouseLeave()
      })

      expect(canvasStore.store.getState().hoveredStepId).toBeNull()
      expect(canvasStore.store.getState().hoveredProtocolId).toBeNull()
    })
  })

  describe('rapid node-to-node hover', () => {
    it('cancels previous timer when entering a new protocol node', () => {
      const { result } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeProtocolNode('proto-1'))
      })

      act(() => {
        vi.advanceTimersByTime(100)
      })

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeProtocolNode('proto-2'))
      })

      act(() => {
        vi.advanceTimersByTime(CANVAS.GROUP_HOVER_DELAY_MS)
      })

      const state = canvasStore.store.getState()
      expect(state.hoveredStepId).toBe('proto-2')
      expect(state.hoveredProtocolId).toBe('proto-2')
    })

    it('cancels previous timer when entering a member node', () => {
      const { result } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeProtocolNode('proto-1'))
      })

      act(() => {
        vi.advanceTimersByTime(100)
      })

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeMemberNode('doc-1'))
      })

      act(() => {
        vi.advanceTimersByTime(CANVAS.GROUP_HOVER_DELAY_MS)
      })

      const state = canvasStore.store.getState()
      expect(state.hoveredStepId).toBe('doc-1')
      expect(state.hoveredProtocolId).toBeNull()
    })
  })

  describe('cleanup', () => {
    it('clears timer on unmount', () => {
      const { result, unmount } = renderHook(() => useGroupHoverDelay())

      act(() => {
        result.current.onNodeMouseEnter(makeMouseEvent(), makeProtocolNode('proto-1'))
      })

      unmount()

      act(() => {
        vi.advanceTimersByTime(CANVAS.GROUP_HOVER_DELAY_MS)
      })

      expect(canvasStore.store.getState().hoveredProtocolId).toBeNull()
    })
  })
})
