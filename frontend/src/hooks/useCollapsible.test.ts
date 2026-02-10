import { describe, it, expect } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useCollapsible } from './useCollapsible'

describe('useCollapsible', () => {
  it('defaults to open', () => {
    const { result } = renderHook(() => useCollapsible())
    expect(result.current.open).toBe(true)
  })

  it('respects defaultOpen = false', () => {
    const { result } = renderHook(() => useCollapsible(false))
    expect(result.current.open).toBe(false)
  })

  it('toggles open state', () => {
    const { result } = renderHook(() => useCollapsible())
    expect(result.current.open).toBe(true)

    act(() => {
      result.current.onToggle()
    })
    expect(result.current.open).toBe(false)

    act(() => {
      result.current.onToggle()
    })
    expect(result.current.open).toBe(true)
  })

  it('returns stable onToggle reference', () => {
    const { result, rerender } = renderHook(() => useCollapsible())
    const firstRef = result.current.onToggle
    rerender()
    expect(result.current.onToggle).toBe(firstRef)
  })
})
