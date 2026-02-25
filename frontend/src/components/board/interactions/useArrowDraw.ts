// ============================================================================
// useArrowDraw — Arrow Drawing Interaction
// ============================================================================

import { useCallback } from 'react'
import { computeBindingAnchor } from '../arrows'
import type { AnchorPoint, BoardElements, InteractionMode, ViewportState } from '../elements'
import { addArrow, createArrow, hitTestBox, screenToCanvas } from '../elements'
import type { SetElements, SetInteraction } from './types'

const useArrowDraw = (
  setElements: SetElements,
  setInteraction: SetInteraction,
  viewport: ViewportState,
  containerRef: React.RefObject<HTMLDivElement | null>,
) => {
  const onArrowStart = useCallback((sourceBoxId: string, anchor: AnchorPoint, e: React.PointerEvent) => {
    const container = containerRef.current
    if (container === null) return

    const rect = container.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    setInteraction({
      type: 'drawing-arrow',
      sourceBoxId,
      sourceAnchor: anchor,
      cursorX: canvas.x,
      cursorY: canvas.y,
    })
  }, [containerRef, setInteraction, viewport])

  const onArrowMove = useCallback((e: React.PointerEvent, interaction: InteractionMode) => {
    if (interaction.type !== 'drawing-arrow') return

    const container = containerRef.current
    if (container === null) return

    const rect = container.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    setInteraction({
      ...interaction,
      cursorX: canvas.x,
      cursorY: canvas.y,
    })
  }, [containerRef, setInteraction, viewport])

  const onArrowEnd = useCallback((e: React.PointerEvent, interaction: InteractionMode, elements: BoardElements) => {
    if (interaction.type !== 'drawing-arrow') return

    const container = containerRef.current
    if (container === null) {
      setInteraction({ type: 'idle' })
      return
    }

    const rect = container.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    // Hit test for target box (excluding source)
    const targetBoxId = hitTestBox(elements, canvas)
    if (targetBoxId !== null && targetBoxId !== interaction.sourceBoxId) {
      const targetBox = elements.boxes.get(targetBoxId)
      if (targetBox !== undefined) {
        const targetAnchor = computeBindingAnchor(targetBox, canvas)
        const arrow = createArrow(
          interaction.sourceBoxId,
          targetBoxId,
          interaction.sourceAnchor,
          targetAnchor,
        )
        setElements((s) => addArrow(s, arrow))
      }
    }

    setInteraction({ type: 'idle' })
  }, [containerRef, setElements, setInteraction, viewport])

  return { onArrowStart, onArrowMove, onArrowEnd } as const
}

export { useArrowDraw }
