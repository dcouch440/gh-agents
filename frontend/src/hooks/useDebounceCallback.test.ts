import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useDebounceCallback } from './useDebounceCallback'

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('useDebounceCallback', () => {
  it('does not call callback immediately', () => {
    const callback = vi.fn()
    const { result } = renderHook(() => useDebounceCallback(callback, 500))

    act(() => {
      result.current('hello')
    })

    expect(callback).not.toHaveBeenCalled()
  })

  it('calls callback after delay', () => {
    const callback = vi.fn()
    const { result } = renderHook(() => useDebounceCallback(callback, 500))

    act(() => {
      result.current('hello')
    })
    act(() => {
      vi.advanceTimersByTime(500)
    })

    expect(callback).toHaveBeenCalledOnce()
    expect(callback).toHaveBeenCalledWith('hello')
  })

  it('only fires the last call when called rapidly', () => {
    const callback = vi.fn()
    const { result } = renderHook(() => useDebounceCallback(callback, 300))

    act(() => {
      result.current('a')
    })
    act(() => {
      vi.advanceTimersByTime(100)
    })
    act(() => {
      result.current('b')
    })
    act(() => {
      vi.advanceTimersByTime(100)
    })
    act(() => {
      result.current('c')
    })
    act(() => {
      vi.advanceTimersByTime(300)
    })

    expect(callback).toHaveBeenCalledOnce()
    expect(callback).toHaveBeenCalledWith('c')
  })

  it('cancels pending callback on unmount', () => {
    const callback = vi.fn()
    const { result, unmount } = renderHook(() => useDebounceCallback(callback, 500))

    act(() => {
      result.current('hello')
    })
    unmount()
    act(() => {
      vi.advanceTimersByTime(500)
    })

    expect(callback).not.toHaveBeenCalled()
  })

  it('uses the latest callback reference', () => {
    const firstCallback = vi.fn()
    const secondCallback = vi.fn()

    const { result, rerender } = renderHook(({ cb }) => useDebounceCallback(cb, 500), { initialProps: { cb: firstCallback } })

    act(() => {
      result.current('hello')
    })
    rerender({ cb: secondCallback })
    act(() => {
      vi.advanceTimersByTime(500)
    })

    expect(firstCallback).not.toHaveBeenCalled()
    expect(secondCallback).toHaveBeenCalledWith('hello')
  })
})
