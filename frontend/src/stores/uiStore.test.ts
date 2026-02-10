// ── Mocks (must be installed before uiStore module loads) ────────────────────

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

const mockMatchMedia = vi.hoisted(() => vi.fn())
let mediaChangeHandler: ((e: MediaQueryListEvent) => void) | null = null

// Install mocks before any module imports run
vi.hoisted(() => {
  Object.defineProperty(globalThis, 'localStorage', {
    value: mockStorage,
    writable: true,
    configurable: true,
  })
})

vi.hoisted(() => {
  mockMatchMedia.mockImplementation((query: string) => ({
    matches: query === '(prefers-color-scheme: dark)',
    media: query,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }))

  Object.defineProperty(globalThis, 'matchMedia', {
    value: mockMatchMedia,
    writable: true,
    configurable: true,
  })
})

// Now import the module — it will use our mocks during initialization
import { uiStore } from './uiStore'
import type { UIState } from './uiStore'

// ── Helpers ──────────────────────────────────────────────────────────────────

const resetStore = () => {
  uiStore.store.setState({
    theme: 'light',
    toasts: [],
    commandPaletteOpen: false,
  })
}

const getState = (): UIState => uiStore.store.getState()

// ── Tests ────────────────────────────────────────────────────────────────────

describe('uiStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStorage.store.clear()
    mediaChangeHandler = null

    Object.defineProperty(window, 'localStorage', {
      value: mockStorage,
      writable: true,
      configurable: true,
    })

    mockMatchMedia.mockImplementation((query: string) => ({
      matches: query === '(prefers-color-scheme: dark)',
      media: query,
      onchange: null,
      addEventListener: vi.fn((_event: string, handler: (e: MediaQueryListEvent) => void) => {
        mediaChangeHandler = handler
      }),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }))

    Object.defineProperty(window, 'matchMedia', {
      value: mockMatchMedia,
      writable: true,
      configurable: true,
    })

    resetStore()
  })

  // ── Theme ──────────────────────────────────────────────────────────────

  describe('theme', () => {
    it('setTheme updates state and persists to localStorage', () => {
      uiStore.setTheme('dark')

      expect(getState().theme).toBe('dark')
      expect(mockStorage.setItem).toHaveBeenCalledWith('nexor_theme', 'dark')
    })

    it('toggleTheme flips light to dark', () => {
      uiStore.store.setState({ theme: 'light' })

      uiStore.toggleTheme()

      expect(getState().theme).toBe('dark')
      expect(mockStorage.setItem).toHaveBeenCalledWith('nexor_theme', 'dark')
    })

    it('toggleTheme flips dark to light', () => {
      uiStore.store.setState({ theme: 'dark' })

      uiStore.toggleTheme()

      expect(getState().theme).toBe('light')
      expect(mockStorage.setItem).toHaveBeenCalledWith('nexor_theme', 'light')
    })

    it('selectTheme returns current theme', () => {
      uiStore.store.setState({ theme: 'dark' })
      expect(uiStore.selectTheme(getState())).toBe('dark')
    })
  })

  // ── Toasts ─────────────────────────────────────────────────────────────

  describe('toasts', () => {
    it('addToast appends toast with generated id', () => {
      const id = uiStore.addToast({ message: 'Hello' })

      expect(id).toMatch(/^toast-\d+$/)
      const toasts = getState().toasts
      expect(toasts).toHaveLength(1)
      expect(toasts[0].message).toBe('Hello')
      expect(toasts[0].type).toBe('info')
      expect(toasts[0].duration).toBe(5000)
      expect(typeof toasts[0].createdAt).toBe('number')
    })

    it('addToast uses custom type and duration', () => {
      uiStore.addToast({ message: 'Error!', type: 'error', duration: 10000 })

      const toast = getState().toasts[0]
      expect(toast.type).toBe('error')
      expect(toast.duration).toBe(10000)
    })

    it('addToast with null duration creates persistent toast', () => {
      uiStore.addToast({ message: 'Persistent', duration: null })

      expect(getState().toasts[0].duration).toBeNull()
    })

    it('dismissToast removes by id', () => {
      const id1 = uiStore.addToast({ message: 'First' })
      const id2 = uiStore.addToast({ message: 'Second' })

      uiStore.dismissToast(id1)

      const toasts = getState().toasts
      expect(toasts).toHaveLength(1)
      expect(toasts[0].id).toBe(id2)
    })

    it('multiple toasts maintain insertion order', () => {
      uiStore.addToast({ message: 'A' })
      uiStore.addToast({ message: 'B' })
      uiStore.addToast({ message: 'C' })

      const messages = getState().toasts.map((t) => t.message)
      expect(messages).toEqual(['A', 'B', 'C'])
    })

    it('dismissToast with unknown id is a no-op', () => {
      uiStore.addToast({ message: 'Keep me' })

      uiStore.dismissToast('nonexistent')

      expect(getState().toasts).toHaveLength(1)
    })

    it('selectToasts returns current toasts', () => {
      uiStore.addToast({ message: 'Test' })
      expect(uiStore.selectToasts(getState())).toHaveLength(1)
    })
  })

  // ── Command Palette ────────────────────────────────────────────────────

  describe('commandPalette', () => {
    it('openCommandPalette sets open to true', () => {
      uiStore.openCommandPalette()
      expect(getState().commandPaletteOpen).toBe(true)
    })

    it('closeCommandPalette sets open to false', () => {
      uiStore.store.setState({ commandPaletteOpen: true })

      uiStore.closeCommandPalette()

      expect(getState().commandPaletteOpen).toBe(false)
    })

    it('toggleCommandPalette flips state', () => {
      expect(getState().commandPaletteOpen).toBe(false)

      uiStore.toggleCommandPalette()
      expect(getState().commandPaletteOpen).toBe(true)

      uiStore.toggleCommandPalette()
      expect(getState().commandPaletteOpen).toBe(false)
    })

    it('selectCommandPaletteOpen returns current value', () => {
      uiStore.store.setState({ commandPaletteOpen: true })
      expect(uiStore.selectCommandPaletteOpen(getState())).toBe(true)
    })
  })

  // ── System theme listener ─────────────────────────────────────────────

  describe('initSystemThemeListener', () => {
    it('updates theme when no stored preference exists', () => {
      uiStore.store.setState({ theme: 'light' })
      const cleanup = uiStore.initSystemThemeListener()

      expect(mediaChangeHandler).not.toBeNull()

      // Simulate system preference changing to dark
      mediaChangeHandler!({ matches: true } as MediaQueryListEvent)

      expect(getState().theme).toBe('dark')

      cleanup()
    })

    it('ignores system change when stored preference exists', () => {
      mockStorage.store.set('nexor_theme', 'light')
      uiStore.store.setState({ theme: 'light' })
      const cleanup = uiStore.initSystemThemeListener()

      mediaChangeHandler!({ matches: true } as MediaQueryListEvent)

      expect(getState().theme).toBe('light')

      cleanup()
    })

    it('returns cleanup function that removes listener', () => {
      const cleanup = uiStore.initSystemThemeListener()

      cleanup()

      // Calling initSystemThemeListener again should work (not deduplicate)
      const cleanup2 = uiStore.initSystemThemeListener()
      expect(cleanup2).toBeInstanceOf(Function)
      cleanup2()
    })

    it('deduplicates calls — second call returns same cleanup', () => {
      const cleanup1 = uiStore.initSystemThemeListener()
      const cleanup2 = uiStore.initSystemThemeListener()

      expect(cleanup1).toBe(cleanup2)

      cleanup1()
    })
  })
})
