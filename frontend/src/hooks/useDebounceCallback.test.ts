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
      result.current.call('hello')
    })

    expect(callback).not.toHaveBeenCalled()
  })

  it('calls callback after delay', () => {
    const callback = vi.fn()
    const { result } = renderHook(() => useDebounceCallback(callback, 500))

    act(() => {
      result.current.call('hello')
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
      result.current.call('a')
    })
    act(() => {
      vi.advanceTimersByTime(100)
    })
    act(() => {
      result.current.call('b')
    })
    act(() => {
      vi.advanceTimersByTime(100)
    })
    act(() => {
      result.current.call('c')
    })
    act(() => {
      vi.advanceTimersByTime(300)
    })

    expect(callback).toHaveBeenCalledOnce()
    expect(callback).toHaveBeenCalledWith('c')
  })

  it('cancels pending callback on unmount by default', () => {
    const callback = vi.fn()
    const { result, unmount } = renderHook(() => useDebounceCallback(callback, 500))

    act(() => {
      result.current.call('hello')
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
      result.current.call('hello')
    })
    rerender({ cb: secondCallback })
    act(() => {
      vi.advanceTimersByTime(500)
    })

    expect(firstCallback).not.toHaveBeenCalled()
    expect(secondCallback).toHaveBeenCalledWith('hello')
  })

  describe('flush', () => {
    it('fires pending callback immediately', () => {
      const callback = vi.fn()
      const { result } = renderHook(() => useDebounceCallback(callback, 500))

      act(() => {
        result.current.call('hello')
      })
      act(() => {
        result.current.flush()
      })

      expect(callback).toHaveBeenCalledOnce()
      expect(callback).toHaveBeenCalledWith('hello')
    })

    it('is a no-op when nothing is pending', () => {
      const callback = vi.fn()
      const { result } = renderHook(() => useDebounceCallback(callback, 500))

      act(() => {
        result.current.flush()
      })

      expect(callback).not.toHaveBeenCalled()
    })

    it('prevents the debounced call from firing again', () => {
      const callback = vi.fn()
      const { result } = renderHook(() => useDebounceCallback(callback, 500))

      act(() => {
        result.current.call('hello')
      })
      act(() => {
        result.current.flush()
      })
      act(() => {
        vi.advanceTimersByTime(500)
      })

      expect(callback).toHaveBeenCalledOnce()
    })
  })

  describe('cancel', () => {
    it('prevents pending callback from firing', () => {
      const callback = vi.fn()
      const { result } = renderHook(() => useDebounceCallback(callback, 500))

      act(() => {
        result.current.call('hello')
      })
      act(() => {
        result.current.cancel()
      })
      act(() => {
        vi.advanceTimersByTime(500)
      })

      expect(callback).not.toHaveBeenCalled()
    })

    it('is a no-op when nothing is pending', () => {
      const callback = vi.fn()
      const { result } = renderHook(() => useDebounceCallback(callback, 500))

      act(() => {
        result.current.cancel()
      })

      expect(callback).not.toHaveBeenCalled()
    })
  })

  describe('flushOnUnmount', () => {
    it('fires pending callback on unmount when enabled', () => {
      const callback = vi.fn()
      const { result, unmount } = renderHook(() =>
        useDebounceCallback(callback, 500, { flushOnUnmount: true }),
      )

      act(() => {
        result.current.call('goodbye')
      })
      unmount()

      expect(callback).toHaveBeenCalledOnce()
      expect(callback).toHaveBeenCalledWith('goodbye')
    })

    it('does not fire on unmount when nothing is pending', () => {
      const callback = vi.fn()
      const { unmount } = renderHook(() =>
        useDebounceCallback(callback, 500, { flushOnUnmount: true }),
      )

      unmount()

      expect(callback).not.toHaveBeenCalled()
    })
  })
})
