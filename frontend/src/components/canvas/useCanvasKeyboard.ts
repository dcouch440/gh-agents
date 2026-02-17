import { useEffect } from 'react'
import { shareStore, focusModeStore } from '@/stores'

/**
 * Registers global keyboard shortcuts for the canvas:
 * - ESC → cancel share mode
 * - Alt+F → enter focus mode
 */
const useCanvasKeyboard = (shareActive: boolean, enterFocusMode: () => void) => {
  useEffect(() => {
    if (!shareActive) return
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        shareStore.cancelShare()
      }
    }
    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [shareActive])

  useEffect(() => {
    const handleFocusKey = (e: KeyboardEvent) => {
      if (e.altKey && (e.key === 'f' || e.key === 'F') && !focusModeStore.store.getState().active) {
        e.preventDefault()
        enterFocusMode()
      }
    }
    document.addEventListener('keydown', handleFocusKey)
    return () => {
      document.removeEventListener('keydown', handleFocusKey)
    }
  }, [enterFocusMode])
}

export { useCanvasKeyboard }
