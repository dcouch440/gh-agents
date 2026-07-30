import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { ElementDebouncerMap } from './elementDebouncer'

describe('ElementDebouncerMap', () => {
  beforeEach(() => { vi.useFakeTimers() })
  afterEach(() => { vi.useRealTimers() })

  it('fires callback after delay', () => {
    const onFire = vi.fn()
    const debouncer = new ElementDebouncerMap<string>(100, onFire)

    debouncer.schedule('a', 'payload-a')
    expect(onFire).not.toHaveBeenCalled()

    vi.advanceTimersByTime(100)
    expect(onFire).toHaveBeenCalledWith('a', 'payload-a')
    expect(onFire).toHaveBeenCalledTimes(1)

    debouncer.dispose()
  })

  it('resets timer on re-schedule with latest payload', () => {
    const onFire = vi.fn()
    const debouncer = new ElementDebouncerMap<string>(100, onFire)

    debouncer.schedule('a', 'first')
    vi.advanceTimersByTime(80)
    debouncer.schedule('a', 'second')
    vi.advanceTimersByTime(80)
    expect(onFire).not.toHaveBeenCalled()

    vi.advanceTimersByTime(20)
    expect(onFire).toHaveBeenCalledWith('a', 'second')
    expect(onFire).toHaveBeenCalledTimes(1)

    debouncer.dispose()
  })

  it('manages independent timers per element', () => {
    const onFire = vi.fn()
    const debouncer = new ElementDebouncerMap<string>(100, onFire)

    debouncer.schedule('a', 'pa')
    debouncer.schedule('b', 'pb')

    vi.advanceTimersByTime(100)
    expect(onFire).toHaveBeenCalledTimes(2)
    expect(onFire).toHaveBeenCalledWith('a', 'pa')
    expect(onFire).toHaveBeenCalledWith('b', 'pb')

    debouncer.dispose()
  })

  it('cancel prevents callback from firing', () => {
    const onFire = vi.fn()
    const debouncer = new ElementDebouncerMap<string>(100, onFire)

    debouncer.schedule('a', 'payload')
    debouncer.cancel('a')

    vi.advanceTimersByTime(200)
    expect(onFire).not.toHaveBeenCalled()
    expect(debouncer.size).toBe(0)

    debouncer.dispose()
  })

  it('cancelAll clears all pending timers', () => {
    const onFire = vi.fn()
    const debouncer = new ElementDebouncerMap<string>(100, onFire)

    debouncer.schedule('a', 'pa')
    debouncer.schedule('b', 'pb')
    debouncer.cancelAll()

    vi.advanceTimersByTime(200)
    expect(onFire).not.toHaveBeenCalled()
    expect(debouncer.size).toBe(0)

    debouncer.dispose()
  })

  it('flushAll fires all pending callbacks immediately', () => {
    const onFire = vi.fn()
    const debouncer = new ElementDebouncerMap<string>(100, onFire)

    debouncer.schedule('a', 'pa')
    debouncer.schedule('b', 'pb')
    debouncer.flushAll()

    expect(onFire).toHaveBeenCalledTimes(2)
    expect(onFire).toHaveBeenCalledWith('a', 'pa')
    expect(onFire).toHaveBeenCalledWith('b', 'pb')
    expect(debouncer.size).toBe(0)

    // No double-fire after timer would have elapsed
    vi.advanceTimersByTime(200)
    expect(onFire).toHaveBeenCalledTimes(2)

    debouncer.dispose()
  })

  it('hasPending returns correct state', () => {
    const onFire = vi.fn()
    const debouncer = new ElementDebouncerMap<string>(100, onFire)

    expect(debouncer.hasPending('a')).toBe(false)
    debouncer.schedule('a', 'pa')
    expect(debouncer.hasPending('a')).toBe(true)

    vi.advanceTimersByTime(100)
    expect(debouncer.hasPending('a')).toBe(false)

    debouncer.dispose()
  })

  it('dispose clears everything', () => {
    const onFire = vi.fn()
    const debouncer = new ElementDebouncerMap<string>(100, onFire)

    debouncer.schedule('a', 'pa')
    debouncer.dispose()

    vi.advanceTimersByTime(200)
    expect(onFire).not.toHaveBeenCalled()
    expect(debouncer.size).toBe(0)
  })
})
