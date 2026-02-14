const mockStorage = vi.hoisted(() => {
  const store = new Map<string, string>()
  return {
    store,
    getItem: vi.fn((key: string) => store.get(key) ?? null),
    setItem: vi.fn((key: string, value: string) => {
      store.set(key, value)
    }),
    removeItem: vi.fn((key: string) => {
      store.delete(key)
    }),
    clear: vi.fn(() => {
      store.clear()
    }),
    get length() {
      return store.size
    },
    key: vi.fn(() => null),
  }
})

vi.hoisted(() => {
  Object.defineProperty(globalThis, 'localStorage', {
    value: mockStorage,
    writable: true,
    configurable: true,
  })
})

import { lsGet, lsSet } from './localStorage'

describe('localStorage helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStorage.store.clear()
  })

  describe('lsGet', () => {
    it('returns stored value', () => {
      mockStorage.store.set('key', 'value')
      expect(lsGet('key')).toBe('value')
      expect(mockStorage.getItem).toHaveBeenCalledWith('key')
    })

    it('returns null for missing key', () => {
      expect(lsGet('nonexistent')).toBeNull()
    })

    it('returns null when localStorage throws', () => {
      mockStorage.getItem.mockImplementationOnce(() => {
        throw new Error('SecurityError')
      })

      expect(lsGet('key')).toBeNull()
    })
  })

  describe('lsSet', () => {
    it('stores a value', () => {
      lsSet('key', 'value')
      expect(mockStorage.setItem).toHaveBeenCalledWith('key', 'value')
      expect(mockStorage.store.get('key')).toBe('value')
    })

    it('does not throw when localStorage throws', () => {
      mockStorage.setItem.mockImplementationOnce(() => {
        throw new Error('QuotaExceededError')
      })

      expect(() => lsSet('key', 'value')).not.toThrow()
    })
  })
})
