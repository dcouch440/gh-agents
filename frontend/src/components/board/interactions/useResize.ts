// ============================================================================
// useResize — Box Resize Interaction
// ============================================================================

import { useCallback } from 'react'
import { BOARD } from '../constants'
import type { BoardElements, InteractionMode, ResizeHandle, ViewportState } from '../elements'
import { screenToCanvas, updateBoxPosition, updateBoxSize } from '../elements'
import type { SetElements, SetInteraction } from './types'

const useResize = (
  setElements: SetElements,
  setInteraction: SetInteraction,
  viewport: ViewportState,
  containerRef: React.RefObject<HTMLDivElement | null>,
) => {
  const onResizeStart = useCallback((boxId: string, handle: ResizeHandle, e: React.PointerEvent, elements: BoardElements) => {
    const container = containerRef.current
    if (container === null) return

    const box = elements.boxes.get(boxId)
    if (box === undefined) return

    const rect = container.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    setInteraction({
      type: 'resizing',
      boxId,
      handle,
      startX: canvas.x,
      startY: canvas.y,
      startBox: { x: box.x, y: box.y, width: box.width, height: box.height },
    })
  }, [containerRef, setInteraction, viewport])

  const onResizeMove = useCallback((e: React.PointerEvent, interaction: InteractionMode) => {
    if (interaction.type !== 'resizing') return

    const container = containerRef.current
    if (container === null) return

    const rect = container.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    const dx = canvas.x - interaction.startX
    const dy = canvas.y - interaction.startY
    const { x: sx, y: sy, width: sw, height: sh } = interaction.startBox
    const handle = interaction.handle

    let newX = sx
    let newY = sy
    let newW = sw
    let newH = sh

    // Horizontal resizing
    if (handle === 'e' || handle === 'ne' || handle === 'se') {
      newW = Math.max(BOARD.MIN_BOX_WIDTH, sw + dx)
    }
    if (handle === 'w' || handle === 'nw' || handle === 'sw') {
      const proposedW = sw - dx
      if (proposedW >= BOARD.MIN_BOX_WIDTH) {
        newW = proposedW
        newX = sx + dx
      }
    }

    // Vertical resizing
    if (handle === 's' || handle === 'se' || handle === 'sw') {
      newH = Math.max(BOARD.MIN_BOX_HEIGHT, sh + dy)
    }
    if (handle === 'n' || handle === 'ne' || handle === 'nw') {
      const proposedH = sh - dy
      if (proposedH >= BOARD.MIN_BOX_HEIGHT) {
        newH = proposedH
        newY = sy + dy
      }
    }

    setElements((s) => {
      let result = updateBoxSize(s, interaction.boxId, newW, newH)
      result = updateBoxPosition(result, interaction.boxId, newX, newY)
      return result
    })
  }, [containerRef, setElements, viewport])

  const onResizeEnd = useCallback(() => {
    setInteraction({ type: 'idle' })
  }, [setInteraction])

  return { onResizeStart, onResizeMove, onResizeEnd } as const
}

export { useResize }
