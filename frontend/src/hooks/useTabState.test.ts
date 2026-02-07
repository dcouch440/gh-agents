import { describe, it, expect } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useTabState } from './useTabState'

describe('useTabState', () => {
  it('initializes with default value', () => {
    const { result } = renderHook(() => useTabState('decomp'))
    expect(result.current.value).toBe('decomp')
  })

  it('updates value on onChange', () => {
    const { result } = renderHook(() => useTabState('decomp'))

    act(() => { result.current.onChange('route') })
    expect(result.current.value).toBe('route')
  })

  it('returns stable onChange reference', () => {
    const { result, rerender } = renderHook(() => useTabState('decomp'))
    const firstRef = result.current.onChange
    rerender()
    expect(result.current.onChange).toBe(firstRef)
  })
})
