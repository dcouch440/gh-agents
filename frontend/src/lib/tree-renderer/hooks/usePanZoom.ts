import { useState, useCallback, useRef } from 'react'
import type { RefObject } from 'react'

type PanZoomState = {
  panX: number
  panY: number
  zoom: number
}

type PanZoomHandlers = {
  onMouseDown: (e: React.MouseEvent) => void
  onMouseMove: (e: React.MouseEvent) => void
  onMouseUp: () => void
  onMouseLeave: () => void
  onWheel: (e: React.WheelEvent) => void
}

type UsePanZoomResult = {
  state: PanZoomState
  handlers: PanZoomHandlers
  svgRef: RefObject<SVGSVGElement | null>
}

const MIN_ZOOM = 0.25
const MAX_ZOOM = 3
const ZOOM_STEP = 0.001

const usePanZoom = (): UsePanZoomResult => {
  const [state, setState] = useState<PanZoomState>({ panX: 0, panY: 0, zoom: 1 })
  const draggingRef = useRef(false)
  const lastPosRef = useRef({ x: 0, y: 0 })
  const svgRef = useRef<SVGSVGElement | null>(null)

  const onMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return
    draggingRef.current = true
    lastPosRef.current = { x: e.clientX, y: e.clientY }
  }, [])

  const onMouseMove = useCallback((e: React.MouseEvent) => {
    if (!draggingRef.current) return
    const dx = e.clientX - lastPosRef.current.x
    const dy = e.clientY - lastPosRef.current.y
    lastPosRef.current = { x: e.clientX, y: e.clientY }
    setState((prev) => ({
      ...prev,
      panX: prev.panX + dx / prev.zoom,
      panY: prev.panY + dy / prev.zoom,
    }))
  }, [])

  const onMouseUp = useCallback(() => {
    draggingRef.current = false
  }, [])

  const onMouseLeave = useCallback(() => {
    draggingRef.current = false
  }, [])

  const onWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault()
    const delta = -e.deltaY * ZOOM_STEP
    setState((prev) => ({
      ...prev,
      zoom: Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, prev.zoom + delta)),
    }))
  }, [])

  return {
    state,
    handlers: { onMouseDown, onMouseMove, onMouseUp, onMouseLeave, onWheel },
    svgRef,
  }
}

export { usePanZoom }
export type { PanZoomState, UsePanZoomResult }
