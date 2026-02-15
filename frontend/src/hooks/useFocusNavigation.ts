import { useEffect, useRef, useCallback } from 'react'
import { useStore } from '@/stores'
import { focusModeStore } from '@/stores/focusModeStore'

// ── Constants ────────────────────────────────────────────────────────────────

const SWIPE_THRESHOLD_PX = 50
const SWIPE_MAX_ANGLE_DEG = 30
const SWIPE_MAX_TIME_MS = 500

// ── Types ────────────────────────────────────────────────────────────────────

type SwipeState = {
  startX: number
  startY: number
  startTime: number
  tracking: boolean
}

// ── Hook ─────────────────────────────────────────────────────────────────────

const useFocusNavigation = () => {
  const active = useStore(focusModeStore.store, focusModeStore.selectActive)
  const expandedArtifact = useStore(focusModeStore.store, focusModeStore.selectExpandedArtifactId)
  const swipeRef = useRef<SwipeState>({ startX: 0, startY: 0, startTime: 0, tracking: false })

  // Keyboard handler
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Alt+F: exit focus mode (entry is handled by the caller)
      if (e.altKey && (e.key === 'f' || e.key === 'F')) {
        if (active) {
          e.preventDefault()
          focusModeStore.exit()
        }
        return
      }

      if (!active) return

      // Escape: collapse artifact first, then exit focus mode
      if (e.key === 'Escape') {
        e.preventDefault()
        if (expandedArtifact !== null) {
          focusModeStore.collapseArtifact()
        } else {
          focusModeStore.exit()
        }
        return
      }

      // Alt+ArrowLeft: previous step
      if (e.altKey && e.key === 'ArrowLeft') {
        e.preventDefault()
        focusModeStore.goPrev()
        return
      }

      // Alt+ArrowRight: next step
      if (e.altKey && e.key === 'ArrowRight') {
        e.preventDefault()
        focusModeStore.goNext()
        return
      }

      // Alt+ArrowDown: collapse artifact detail
      if (e.altKey && e.key === 'ArrowDown') {
        e.preventDefault()
        focusModeStore.collapseArtifact()
        return
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [active, expandedArtifact])

  // Touch: start tracking
  const onTouchStart = useCallback(
    (e: React.TouchEvent) => {
      if (!active || e.touches.length !== 1) return
      const touch = e.touches[0]!
      swipeRef.current = {
        startX: touch.clientX,
        startY: touch.clientY,
        startTime: Date.now(),
        tracking: true,
      }
    },
    [active],
  )

  // Touch: evaluate swipe
  const onTouchEnd = useCallback((e: React.TouchEvent) => {
    const state = swipeRef.current
    if (!state.tracking || e.changedTouches.length !== 1) return
    state.tracking = false

    const touch = e.changedTouches[0]!
    const dx = touch.clientX - state.startX
    const dy = touch.clientY - state.startY
    const elapsed = Date.now() - state.startTime

    if (elapsed > SWIPE_MAX_TIME_MS) return
    if (Math.abs(dx) < SWIPE_THRESHOLD_PX) return

    const angleDeg = Math.abs(Math.atan2(dy, dx) * (180 / Math.PI))
    if (angleDeg > SWIPE_MAX_ANGLE_DEG && angleDeg < 180 - SWIPE_MAX_ANGLE_DEG) return

    if (dx < 0) {
      focusModeStore.goNext()
    } else {
      focusModeStore.goPrev()
    }
  }, [])

  return {
    touchHandlers: { onTouchStart, onTouchEnd },
  }
}

export { useFocusNavigation }
