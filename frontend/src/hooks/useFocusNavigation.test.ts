import { renderHook } from '@testing-library/react'
import { useFocusNavigation } from './useFocusNavigation'

const { mockStore } = vi.hoisted(() => {
  const state = { active: false, expandedArtifactId: null as string | null }
  return {
    mockStore: {
      store: { getState: () => state, subscribe: vi.fn(() => vi.fn()) },
      selectActive: (s: typeof state) => s.active,
      selectExpandedArtifactId: (s: typeof state) => s.expandedArtifactId,
      enter: vi.fn(),
      exit: vi.fn(),
      goNext: vi.fn(),
      goPrev: vi.fn(),
      goToIndex: vi.fn(),
      expandArtifact: vi.fn(),
      collapseArtifact: vi.fn(),
      setActiveTab: vi.fn(),
      _state: state,
    },
  }
})

vi.mock('@/stores', () => ({
  useStore: (store: unknown, selector: (s: unknown) => unknown) => {
    if (store === mockStore.store) return selector(mockStore._state)
    return null
  },
}))

vi.mock('@/stores/focusModeStore', () => ({
  focusModeStore: mockStore,
}))

const fireKey = (key: string, opts: Partial<KeyboardEvent> = {}) => {
  const event = new KeyboardEvent('keydown', {
    key,
    bubbles: true,
    cancelable: true,
    ...opts,
  })
  document.dispatchEvent(event)
}

describe('useFocusNavigation', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStore._state.active = true
    mockStore._state.expandedArtifactId = null
  })

  describe('keyboard navigation', () => {
    it('Alt+ArrowLeft calls goPrev', () => {
      renderHook(() => useFocusNavigation())
      fireKey('ArrowLeft', { altKey: true })
      expect(mockStore.goPrev).toHaveBeenCalledTimes(1)
    })

    it('Alt+ArrowRight calls goNext', () => {
      renderHook(() => useFocusNavigation())
      fireKey('ArrowRight', { altKey: true })
      expect(mockStore.goNext).toHaveBeenCalledTimes(1)
    })

    it('Alt+F calls exit when active', () => {
      renderHook(() => useFocusNavigation())
      fireKey('f', { altKey: true })
      expect(mockStore.exit).toHaveBeenCalledTimes(1)
    })

    it('Alt+F does not call exit when inactive', () => {
      mockStore._state.active = false
      renderHook(() => useFocusNavigation())
      fireKey('f', { altKey: true })
      expect(mockStore.exit).not.toHaveBeenCalled()
    })

    it('Escape with expanded artifact calls collapseArtifact', () => {
      mockStore._state.expandedArtifactId = 'art-1'
      renderHook(() => useFocusNavigation())
      fireKey('Escape')
      expect(mockStore.collapseArtifact).toHaveBeenCalledTimes(1)
      expect(mockStore.exit).not.toHaveBeenCalled()
    })

    it('Escape without expanded artifact calls exit', () => {
      renderHook(() => useFocusNavigation())
      fireKey('Escape')
      expect(mockStore.exit).toHaveBeenCalledTimes(1)
      expect(mockStore.collapseArtifact).not.toHaveBeenCalled()
    })

    it('Alt+ArrowDown calls collapseArtifact', () => {
      renderHook(() => useFocusNavigation())
      fireKey('ArrowDown', { altKey: true })
      expect(mockStore.collapseArtifact).toHaveBeenCalledTimes(1)
    })

    it('does not respond to keyboard when inactive (except Alt+F)', () => {
      mockStore._state.active = false
      renderHook(() => useFocusNavigation())
      fireKey('Escape')
      fireKey('ArrowLeft', { altKey: true })
      fireKey('ArrowRight', { altKey: true })
      expect(mockStore.exit).not.toHaveBeenCalled()
      expect(mockStore.goPrev).not.toHaveBeenCalled()
      expect(mockStore.goNext).not.toHaveBeenCalled()
    })
  })

  describe('touch handlers', () => {
    it('returns onTouchStart and onTouchEnd handlers', () => {
      const { result } = renderHook(() => useFocusNavigation())
      expect(result.current.touchHandlers.onTouchStart).toBeDefined()
      expect(result.current.touchHandlers.onTouchEnd).toBeDefined()
    })
  })
})
