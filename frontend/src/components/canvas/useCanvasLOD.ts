import { useRef, useCallback } from 'react'
import { useStore } from '@xyflow/react'
import type { ReactFlowState } from '@xyflow/react'
import { LOD, DetailLevel } from './constants'

/**
 * Returns the current canvas detail level based on zoom.
 * Uses hysteresis (0.30 down / 0.35 up) to prevent flickering.
 *
 * Only triggers re-renders when the detail level changes,
 * not on every zoom/pan event.
 */
const useCanvasLOD = (): DetailLevel => {
  const levelRef = useRef<DetailLevel>(DetailLevel.FULL)

  const selector = useCallback((state: ReactFlowState): DetailLevel => {
    const zoom = state.transform[2]
    const current = levelRef.current

    if (current === DetailLevel.FULL && zoom < LOD.THRESHOLD_DOWN) {
      levelRef.current = DetailLevel.MINIMAL
      return DetailLevel.MINIMAL
    }
    if (current === DetailLevel.MINIMAL && zoom > LOD.THRESHOLD_UP) {
      levelRef.current = DetailLevel.FULL
      return DetailLevel.FULL
    }
    return current
  }, [])

  return useStore(selector)
}

export { useCanvasLOD }
