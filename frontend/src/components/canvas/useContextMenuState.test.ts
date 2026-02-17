import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useContextMenuState } from './useContextMenuState'

vi.mock('@/stores', () => ({
  shareStore: {
    store: {
      getState: vi.fn(() => ({ active: false })),
    },
  },
}))

const { shareStore } = await import('@/stores')

describe('useContextMenuState', () => {
  const screenToFlowPosition = vi.fn((pos: { x: number; y: number }) => ({ x: pos.x + 100, y: pos.y + 100 }))

  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(shareStore.store.getState).mockReturnValue({ active: false } as ReturnType<typeof shareStore.store.getState>)
  })

  it('starts with null context menu', () => {
    const { result } = renderHook(() => useContextMenuState(screenToFlowPosition))
    expect(result.current.contextMenu).toBeNull()
  })

  describe('onPaneContextMenu', () => {
    it('opens menu with flow-translated coordinates', () => {
      const { result } = renderHook(() => useContextMenuState(screenToFlowPosition))

      act(() => {
        result.current.onPaneContextMenu({
          preventDefault: vi.fn(),
          clientX: 200,
          clientY: 300,
        } as unknown as React.MouseEvent)
      })

      expect(result.current.contextMenu).toEqual({
        x: 200,
        y: 300,
        flowX: 300,
        flowY: 400,
      })
    })

    it('does not open menu when share is active', () => {
      vi.mocked(shareStore.store.getState).mockReturnValue({ active: true } as ReturnType<typeof shareStore.store.getState>)
      const { result } = renderHook(() => useContextMenuState(screenToFlowPosition))

      act(() => {
        result.current.onPaneContextMenu({
          preventDefault: vi.fn(),
          clientX: 200,
          clientY: 300,
        } as unknown as React.MouseEvent)
      })

      expect(result.current.contextMenu).toBeNull()
    })
  })

  describe('onNodeContextMenu', () => {
    it('opens menu with node id and position', () => {
      const { result } = renderHook(() => useContextMenuState(screenToFlowPosition))

      act(() => {
        result.current.onNodeContextMenu(
          { preventDefault: vi.fn() } as unknown as React.MouseEvent,
          { id: 'node-1', position: { x: 50, y: 75 } },
        )
      })

      expect(result.current.contextMenu).not.toBeNull()
      expect(result.current.contextMenu!.flowX).toBe(50)
      expect(result.current.contextMenu!.flowY).toBe(75)
      expect(result.current.contextMenu!.nodeId).toBe('node-1')
    })
  })

  describe('closeMenu', () => {
    it('resets context menu to null', () => {
      const { result } = renderHook(() => useContextMenuState(screenToFlowPosition))

      act(() => {
        result.current.onPaneContextMenu({
          preventDefault: vi.fn(),
          clientX: 200,
          clientY: 300,
        } as unknown as React.MouseEvent)
      })
      expect(result.current.contextMenu).not.toBeNull()

      act(() => {
        result.current.closeMenu()
      })
      expect(result.current.contextMenu).toBeNull()
    })
  })

  describe('onCanvasMouseDown', () => {
    it('closes the menu', () => {
      const { result } = renderHook(() => useContextMenuState(screenToFlowPosition))

      act(() => {
        result.current.onPaneContextMenu({
          preventDefault: vi.fn(),
          clientX: 200,
          clientY: 300,
        } as unknown as React.MouseEvent)
      })
      expect(result.current.contextMenu).not.toBeNull()

      act(() => {
        result.current.onCanvasMouseDown()
      })
      expect(result.current.contextMenu).toBeNull()
    })
  })
})
