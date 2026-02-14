import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useReducedMotion } from './useReducedMotion'

type ChangeHandler = (e: { matches: boolean }) => void

const createMockMediaQuery = (matches: boolean) => {
  let handler: ChangeHandler | null = null
  return {
    matches,
    addEventListener: vi.fn((_event: string, fn: ChangeHandler) => {
      handler = fn
    }),
    removeEventListener: vi.fn((_event: string, _fn: ChangeHandler) => {
      handler = null
    }),
    dispatchChange: (newMatches: boolean) => {
      handler?.({ matches: newMatches })
    },
  }
}

let mockMql: ReturnType<typeof createMockMediaQuery>

beforeEach(() => {
  vi.clearAllMocks()
  mockMql = createMockMediaQuery(false)
  vi.spyOn(window, 'matchMedia').mockReturnValue(mockMql as unknown as MediaQueryList)
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('useReducedMotion', () => {
  it('returns false when prefers-reduced-motion is not set', () => {
    const { result } = renderHook(() => useReducedMotion())
    expect(result.current).toBe(false)
  })

  it('returns true when prefers-reduced-motion: reduce is active', () => {
    mockMql = createMockMediaQuery(true)
    vi.spyOn(window, 'matchMedia').mockReturnValue(mockMql as unknown as MediaQueryList)

    const { result } = renderHook(() => useReducedMotion())
    expect(result.current).toBe(true)
  })

  it('updates when media query changes', () => {
    const { result } = renderHook(() => useReducedMotion())
    expect(result.current).toBe(false)

    act(() => {
      mockMql.dispatchChange(true)
    })
    expect(result.current).toBe(true)

    act(() => {
      mockMql.dispatchChange(false)
    })
    expect(result.current).toBe(false)
  })

  it('registers and cleans up event listener', () => {
    const { unmount } = renderHook(() => useReducedMotion())
    expect(mockMql.addEventListener).toHaveBeenCalledWith('change', expect.any(Function))

    unmount()
    expect(mockMql.removeEventListener).toHaveBeenCalledWith('change', expect.any(Function))
  })
})
