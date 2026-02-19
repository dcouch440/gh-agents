import { useRef, useCallback, useEffect } from 'react'
import { canvasStore } from '@/stores'
import { CANVAS } from './constants'

type NodeHoverInfo = {
  id: string
  data: Record<string, unknown>
}

type GroupHoverCallbacks = {
  onNodeMouseEnter: (_event: React.MouseEvent, node: NodeHoverInfo) => void
  onNodeMouseLeave: () => void
}

/** Module-level drag flag — suppresses hover events during node drag to prevent re-renders. */
let dragging = false

const setDragging = (value: boolean): void => {
  dragging = value
}

const useGroupHoverDelay = (): GroupHoverCallbacks => {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current)
      }
    }
  }, [])

  const onNodeMouseEnter = useCallback((_event: React.MouseEvent, node: NodeHoverInfo) => {
    if (dragging) return

    if (timerRef.current !== null) {
      clearTimeout(timerRef.current)
      timerRef.current = null
    }

    canvasStore.setHoveredStep(node.id)

    if (node.data.isProtocol === true) {
      timerRef.current = setTimeout(() => {
        canvasStore.setHoveredStep(node.id, node.id)
        timerRef.current = null
      }, CANVAS.GROUP_HOVER_DELAY_MS)
    }
  }, [])

  const onNodeMouseLeave = useCallback(() => {
    if (dragging) return

    if (timerRef.current !== null) {
      clearTimeout(timerRef.current)
      timerRef.current = null
    }
    canvasStore.setHoveredStep(null)
  }, [])

  return { onNodeMouseEnter, onNodeMouseLeave }
}

export { useGroupHoverDelay, setDragging }
