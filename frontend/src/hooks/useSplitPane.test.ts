import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useSplitPane } from './useSplitPane'

describe('useSplitPane', () => {
  it('returns initial splitPercent matching opts.initial', () => {
    const { result } = renderHook(() => useSplitPane({ initial: 40, min: 20, max: 80 }))
    expect(result.current.splitPercent).toBe(40)
  })

  it('returns handleMouseDown as a function', () => {
    const { result } = renderHook(() => useSplitPane({ initial: 50, min: 10, max: 90 }))
    expect(typeof result.current.handleMouseDown).toBe('function')
  })
})
