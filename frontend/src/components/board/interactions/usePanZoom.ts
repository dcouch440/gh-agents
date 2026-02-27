// ============================================================================
// usePanZoom — Canvas Pan and Zoom Interaction
// ============================================================================

import { useCallback } from 'react'
import { Geometry } from '@/utils/geometry'
import { BOARD } from '../constants'
import type { ViewportState } from '../elements'

type SetViewport = (fn: (v: ViewportState) => ViewportState) => void

/**
 * Handles canvas pan and zoom via mouse wheel.
 *
 * - Plain wheel: pan
 * - Ctrl/Meta + wheel: zoom centered on cursor
 * - Pinch gesture (trackpad): zoom via ctrlKey
 */
const usePanZoom = (
  viewport: ViewportState,
  setViewport: SetViewport,
) => {
  const onWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault()

    if (e.ctrlKey || e.metaKey) {
      // Read DOM values before the updater — React nullifies e.currentTarget after the handler returns
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
      const cursorX = e.clientX - rect.left
      const cursorY = e.clientY - rect.top
      const deltaY = e.deltaY

      // Zoom centered on cursor
      setViewport((v) => {
        const newZoom = Geometry.clamp(
          v.zoom * (1 - deltaY * BOARD.ZOOM_SPEED),
          BOARD.MIN_ZOOM,
          BOARD.MAX_ZOOM,
        )

        const scale = newZoom / v.zoom
        const panX = cursorX - (cursorX - v.panX) * scale
        const panY = cursorY - (cursorY - v.panY) * scale

        return { panX, panY, zoom: newZoom }
      })
    } else {
      // Pan
      setViewport((v) => ({
        ...v,
        panX: v.panX - e.deltaX,
        panY: v.panY - e.deltaY,
      }))
    }
  }, [setViewport])

  const zoomIn = useCallback(() => {
    setViewport((v) => ({
      ...v,
      zoom: Geometry.clamp(v.zoom * BOARD.ZOOM_BUTTON_STEP, BOARD.MIN_ZOOM, BOARD.MAX_ZOOM),
    }))
  }, [setViewport])

  const zoomOut = useCallback(() => {
    setViewport((v) => ({
      ...v,
      zoom: Geometry.clamp(v.zoom / BOARD.ZOOM_BUTTON_STEP, BOARD.MIN_ZOOM, BOARD.MAX_ZOOM),
    }))
  }, [setViewport])

  const resetZoom = useCallback(() => {
    setViewport(() => ({ panX: 0, panY: 0, zoom: 1 }))
  }, [setViewport])

  return { onWheel, zoomIn, zoomOut, resetZoom } as const
}

export { usePanZoom }
