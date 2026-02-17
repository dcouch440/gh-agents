import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useCanvasKeyboard } from './useCanvasKeyboard'

vi.mock('@/stores', () => {
  const cancelShare = vi.fn()
  return {
    shareStore: {
      cancelShare,
    },
    focusModeStore: {
      store: {
        getState: vi.fn(() => ({ active: false })),
      },
    },
  }
})

const { shareStore, focusModeStore } = await import('@/stores')

describe('useCanvasKeyboard', () => {
  const enterFocusMode = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('ESC to cancel share', () => {
    it('calls cancelShare on ESC when share is active', () => {
      renderHook(() => useCanvasKeyboard(true, enterFocusMode))

      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))

      expect(shareStore.cancelShare).toHaveBeenCalledOnce()
    })

    it('does not listen for ESC when share is inactive', () => {
      renderHook(() => useCanvasKeyboard(false, enterFocusMode))

      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))

      expect(shareStore.cancelShare).not.toHaveBeenCalled()
    })
  })

  describe('Alt+F to enter focus mode', () => {
    it('calls enterFocusMode on Alt+F', () => {
      renderHook(() => useCanvasKeyboard(false, enterFocusMode))

      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'f', altKey: true }))

      expect(enterFocusMode).toHaveBeenCalledOnce()
    })

    it('calls enterFocusMode on Alt+Shift+F', () => {
      renderHook(() => useCanvasKeyboard(false, enterFocusMode))

      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'F', altKey: true }))

      expect(enterFocusMode).toHaveBeenCalledOnce()
    })

    it('does not call enterFocusMode when focus mode is already active', () => {
      vi.mocked(focusModeStore.store.getState).mockReturnValue({ active: true } as ReturnType<typeof focusModeStore.store.getState>)

      renderHook(() => useCanvasKeyboard(false, enterFocusMode))

      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'f', altKey: true }))

      expect(enterFocusMode).not.toHaveBeenCalled()
    })

    it('does not call enterFocusMode on plain F (no Alt)', () => {
      renderHook(() => useCanvasKeyboard(false, enterFocusMode))

      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'f', altKey: false }))

      expect(enterFocusMode).not.toHaveBeenCalled()
    })
  })

  it('cleans up listeners on unmount', () => {
    const spy = vi.spyOn(document, 'removeEventListener')
    const { unmount } = renderHook(() => useCanvasKeyboard(true, enterFocusMode))

    unmount()

    const removedTypes = spy.mock.calls.map((c) => c[0])
    expect(removedTypes).toContain('keydown')
    spy.mockRestore()
  })
})
