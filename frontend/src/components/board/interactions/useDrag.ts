// ============================================================================
// useDrag — Element Dragging Interaction
// ============================================================================

import { useCallback } from 'react'
import { Geometry } from '@/utils/geometry'
import { BOARD } from '../constants'
import type { BoardElements, InteractionMode, ViewportState } from '../elements'
import { screenToCanvas, updateBoxPosition } from '../elements'
import type { SetElements, SetInteraction } from './types'

const useDrag = (
  setElements: SetElements,
  setInteraction: SetInteraction,
  viewport: ViewportState,
  containerRef: React.RefObject<HTMLDivElement | null>,
) => {
  const onDragStart = useCallback((elementId: string, e: React.PointerEvent, elements: BoardElements) => {
    const container = containerRef.current
    if (container === null) return

    const rect = container.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)
    const box = elements.boxes.get(elementId)
    if (box === undefined) return

    const offsetX = canvas.x - box.x
    const offsetY = canvas.y - box.y

    setInteraction({ type: 'dragging', elementId, offsetX, offsetY })
  }, [containerRef, setInteraction, viewport])

  const onDragMove = useCallback((e: React.PointerEvent, interaction: InteractionMode) => {
    if (interaction.type !== 'dragging') return

    const container = containerRef.current
    if (container === null) return

    const rect = container.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    const rawX = canvas.x - interaction.offsetX
    const rawY = canvas.y - interaction.offsetY
    const x = Geometry.snapToGrid(rawX, BOARD.GRID_SIZE)
    const y = Geometry.snapToGrid(rawY, BOARD.GRID_SIZE)

    setElements((s) => updateBoxPosition(s, interaction.elementId, x, y))
  }, [containerRef, setElements, viewport])

  const onDragEnd = useCallback(() => {
    setInteraction({ type: 'idle' })
  }, [setInteraction])

  return { onDragStart, onDragMove, onDragEnd } as const
}

export { useDrag }
