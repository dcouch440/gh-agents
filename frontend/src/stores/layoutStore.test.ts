// ── Mocks (must be installed before layoutStore module loads) ─────────────────

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

import { layoutStore } from './layoutStore'
import type { LayoutState } from './layoutStore'

const getState = (): LayoutState => layoutStore.store.getState()

describe('layoutStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStorage.store.clear()
    layoutStore.store.setState({
      leftPanelOpen: false,
      leftPanelSection: null,
      rightPanelOpen: false,
      rightPanelSection: null,
      rightPanelWidth: 220,
      rightPanelDragging: false,
    })
  })

  describe('initial state', () => {
    it('defaults to panels closed with default width', () => {
      expect(getState().leftPanelOpen).toBe(false)
      expect(getState().leftPanelSection).toBe(null)
      expect(getState().rightPanelOpen).toBe(false)
      expect(getState().rightPanelSection).toBe(null)
      expect(getState().rightPanelWidth).toBe(220)
    })
  })

  describe('selectors', () => {
    it('selectLeftPanelOpen returns current value', () => {
      layoutStore.store.setState({ leftPanelOpen: true })
      expect(layoutStore.selectLeftPanelOpen(getState())).toBe(true)
    })

    it('selectLeftPanelSection returns current section', () => {
      layoutStore.store.setState({ leftPanelSection: '/agents' })
      expect(layoutStore.selectLeftPanelSection(getState())).toBe('/agents')
    })

    it('selectRightPanelOpen returns current value', () => {
      layoutStore.store.setState({ rightPanelOpen: true })
      expect(layoutStore.selectRightPanelOpen(getState())).toBe(true)
    })

    it('selectRightPanelSection returns current section', () => {
      layoutStore.store.setState({ rightPanelSection: 'properties' })
      expect(layoutStore.selectRightPanelSection(getState())).toBe('properties')
    })
  })

  describe('left panel actions', () => {
    it('openLeftPanel sets open state and section, persists to localStorage', () => {
      layoutStore.openLeftPanel('/agents')

      expect(getState().leftPanelOpen).toBe(true)
      expect(getState().leftPanelSection).toBe('/agents')
      expect(mockStorage.setItem).toHaveBeenCalledWith('nexor_left_panel_open', 'true')
      expect(mockStorage.setItem).toHaveBeenCalledWith('nexor_left_panel_section', '/agents')
    })

    it('closeLeftPanel sets open to false, persists to localStorage', () => {
      layoutStore.openLeftPanel('/agents')
      vi.clearAllMocks()

      layoutStore.closeLeftPanel()

      expect(getState().leftPanelOpen).toBe(false)
      expect(mockStorage.setItem).toHaveBeenCalledWith('nexor_left_panel_open', 'false')
    })

    it('toggleLeftPanel opens when closed', () => {
      layoutStore.toggleLeftPanel('/agents')

      expect(getState().leftPanelOpen).toBe(true)
      expect(getState().leftPanelSection).toBe('/agents')
    })

    it('toggleLeftPanel closes when open with same section', () => {
      layoutStore.openLeftPanel('/agents')

      layoutStore.toggleLeftPanel('/agents')

      expect(getState().leftPanelOpen).toBe(false)
    })

    it('toggleLeftPanel switches section when open with different section', () => {
      layoutStore.openLeftPanel('/agents')

      layoutStore.toggleLeftPanel('/tasks')

      expect(getState().leftPanelOpen).toBe(true)
      expect(getState().leftPanelSection).toBe('/tasks')
    })
  })

  describe('right panel actions', () => {
    it('openRightPanel sets open state and section', () => {
      layoutStore.openRightPanel('properties')

      expect(getState().rightPanelOpen).toBe(true)
      expect(getState().rightPanelSection).toBe('properties')
    })

    it('closeRightPanel sets open to false', () => {
      layoutStore.openRightPanel('properties')
      layoutStore.closeRightPanel()

      expect(getState().rightPanelOpen).toBe(false)
    })

    it('toggleRightPanel opens when closed', () => {
      layoutStore.toggleRightPanel('properties')

      expect(getState().rightPanelOpen).toBe(true)
      expect(getState().rightPanelSection).toBe('properties')
    })

    it('toggleRightPanel closes when open with same section', () => {
      layoutStore.openRightPanel('properties')
      layoutStore.toggleRightPanel('properties')

      expect(getState().rightPanelOpen).toBe(false)
    })

    it('toggleRightPanel switches section when open with different section', () => {
      layoutStore.openRightPanel('properties')
      layoutStore.toggleRightPanel('layers')

      expect(getState().rightPanelOpen).toBe(true)
      expect(getState().rightPanelSection).toBe('layers')
    })

    it('openRightPanelIfClosed opens panel when closed', () => {
      layoutStore.openRightPanelIfClosed('properties')

      expect(getState().rightPanelOpen).toBe(true)
      expect(getState().rightPanelSection).toBe('properties')
    })

    it('openRightPanelIfClosed does nothing when already open', () => {
      layoutStore.openRightPanel('agents')
      layoutStore.openRightPanelIfClosed('properties')

      expect(getState().rightPanelOpen).toBe(true)
      expect(getState().rightPanelSection).toBe('agents')
    })
  })

  describe('right panel width', () => {
    it('selectRightPanelWidth returns current width', () => {
      expect(layoutStore.selectRightPanelWidth(getState())).toBe(220)
    })

    it('setRightPanelWidth updates width and persists to localStorage', () => {
      layoutStore.setRightPanelWidth(350)

      expect(getState().rightPanelWidth).toBe(350)
      expect(mockStorage.setItem).toHaveBeenCalledWith('nexor_right_panel_width', '350')
    })

    it('clamps width to minimum', () => {
      layoutStore.setRightPanelWidth(50)

      expect(getState().rightPanelWidth).toBe(240)
    })

    it('clamps width to maximum', () => {
      layoutStore.setRightPanelWidth(999)

      expect(getState().rightPanelWidth).toBe(480)
    })
  })

  describe('right panel dragging', () => {
    it('startRightPanelDrag sets dragging to true', () => {
      layoutStore.startRightPanelDrag()

      expect(getState().rightPanelDragging).toBe(true)
    })

    it('stopRightPanelDrag sets dragging to false', () => {
      layoutStore.startRightPanelDrag()
      layoutStore.stopRightPanelDrag()

      expect(getState().rightPanelDragging).toBe(false)
    })

    it('selectRightPanelDragging returns current value', () => {
      layoutStore.startRightPanelDrag()

      expect(layoutStore.selectRightPanelDragging(getState())).toBe(true)
    })
  })
})
