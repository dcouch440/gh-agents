import { batch, scheduleBatchNotify } from './batch'

describe('batch', () => {
  it('coalesces multiple notifications into one', () => {
    const notify = vi.fn()

    batch(() => {
      scheduleBatchNotify(notify)
      scheduleBatchNotify(notify)
      scheduleBatchNotify(notify)
      expect(notify).not.toHaveBeenCalled()
    })

    // Set deduplicates, so only 1 call (same fn reference)
    expect(notify).toHaveBeenCalledTimes(1)
  })

  it('coalesces distinct notifications into single flush', () => {
    const notify1 = vi.fn()
    const notify2 = vi.fn()

    batch(() => {
      scheduleBatchNotify(notify1)
      scheduleBatchNotify(notify2)
      expect(notify1).not.toHaveBeenCalled()
      expect(notify2).not.toHaveBeenCalled()
    })

    expect(notify1).toHaveBeenCalledTimes(1)
    expect(notify2).toHaveBeenCalledTimes(1)
  })

  it('without batch, scheduleBatchNotify fires immediately', () => {
    const notify = vi.fn()

    scheduleBatchNotify(notify)
    expect(notify).toHaveBeenCalledTimes(1)

    scheduleBatchNotify(notify)
    expect(notify).toHaveBeenCalledTimes(2)
  })

  it('nested batch only flushes at outermost level', () => {
    const notify = vi.fn()

    batch(() => {
      scheduleBatchNotify(notify)

      batch(() => {
        scheduleBatchNotify(notify)
        expect(notify).not.toHaveBeenCalled()
      })

      // Inner batch ended but outer still active — not flushed
      expect(notify).not.toHaveBeenCalled()
    })

    expect(notify).toHaveBeenCalledTimes(1) // Deduped by Set
  })

  it('flushes even if fn throws', () => {
    const notify = vi.fn()

    expect(() => {
      batch(() => {
        scheduleBatchNotify(notify)
        throw new Error('boom')
      })
    }).toThrow('boom')

    expect(notify).toHaveBeenCalledTimes(1)
  })
})
