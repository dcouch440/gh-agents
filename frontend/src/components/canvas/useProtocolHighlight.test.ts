import { describe, it, expect, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useProtocolHighlight } from './useProtocolHighlight'
import { canvasStore } from '@/stores'

describe('useProtocolHighlight', () => {
  beforeEach(() => {
    canvasStore.reset()
  })

  it('returns none when protocolStepId is null', () => {
    const { result } = renderHook(() => useProtocolHighlight(null))
    expect(result.current).toBe('none')
  })

  it('returns none when protocol is not hovered or selected', () => {
    const { result } = renderHook(() => useProtocolHighlight('proto-1'))
    expect(result.current).toBe('none')
  })

  it('returns hover when protocol step is hovered', () => {
    const { result } = renderHook(() => useProtocolHighlight('proto-1'))
    act(() => {
      canvasStore.setHoveredStep('proto-1')
    })
    expect(result.current).toBe('hover')
  })

  it('returns none when a different step is hovered', () => {
    const { result } = renderHook(() => useProtocolHighlight('proto-1'))
    act(() => {
      canvasStore.setHoveredStep('other-step')
    })
    expect(result.current).toBe('none')
  })

  it('returns select when protocol step is in selected set', () => {
    const { result } = renderHook(() => useProtocolHighlight('proto-1'))
    act(() => {
      canvasStore.selectSteps(['proto-1'])
    })
    expect(result.current).toBe('select')
  })

  it('prioritizes select over hover', () => {
    const { result } = renderHook(() => useProtocolHighlight('proto-1'))
    act(() => {
      canvasStore.selectSteps(['proto-1'])
      canvasStore.setHoveredStep('proto-1')
    })
    expect(result.current).toBe('select')
  })

  it('transitions back to none when hover clears', () => {
    const { result } = renderHook(() => useProtocolHighlight('proto-1'))
    act(() => {
      canvasStore.setHoveredStep('proto-1')
    })
    expect(result.current).toBe('hover')
    act(() => {
      canvasStore.setHoveredStep(null)
    })
    expect(result.current).toBe('none')
  })

  it('returns select when protocol step is in highlightedProtocolStepIds', () => {
    const { result } = renderHook(() => useProtocolHighlight('proto-1'))
    act(() => {
      canvasStore.setHighlightedProtocols(new Set(['proto-1']))
    })
    expect(result.current).toBe('select')
  })

  it('returns none when a different protocol is highlighted', () => {
    const { result } = renderHook(() => useProtocolHighlight('proto-1'))
    act(() => {
      canvasStore.setHighlightedProtocols(new Set(['proto-2']))
    })
    expect(result.current).toBe('none')
  })
})
