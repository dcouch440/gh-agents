// ============================================================================
// useArrowDraw — Arrow Drawing Interaction
// ============================================================================

import { useCallback } from 'react'
import { anchorToFocus, computeGeometricFocus, focusToAbsolute } from '../arrows'
import type { AnchorPoint, BoardElements, InteractionMode, ViewportState } from '../elements'
import { addArrow, containerEventToCanvas, createArrow, hasArrow, hitTestBox } from '../elements'
import type { SetElements, SetInteraction } from './types'

const useArrowDraw = (
  setElements: SetElements,
  setInteraction: SetInteraction,
  viewport: ViewportState,
  containerRef: React.RefObject<HTMLDivElement | null>,
) => {
  const onArrowStart = useCallback((sourceBoxId: string, anchor: AnchorPoint, e: React.PointerEvent) => {
    const canvas = containerEventToCanvas(containerRef, e, viewport)
    if (canvas === null) return

    setInteraction({
      type: 'drawing-arrow',
      sourceBoxId,
      sourceFocus: anchorToFocus(anchor),
      cursorX: canvas.x,
      cursorY: canvas.y,
    })
  }, [containerRef, setInteraction, viewport])

  const onArrowMove = useCallback((e: React.PointerEvent, interaction: InteractionMode) => {
    if (interaction.type !== 'drawing-arrow') return

    const canvas = containerEventToCanvas(containerRef, e, viewport)
    if (canvas === null) return

    setInteraction({
      ...interaction,
      cursorX: canvas.x,
      cursorY: canvas.y,
    })
  }, [containerRef, setInteraction, viewport])

  const onArrowEnd = useCallback((e: React.PointerEvent, interaction: InteractionMode, elements: BoardElements) => {
    if (interaction.type !== 'drawing-arrow') return

    const canvas = containerEventToCanvas(containerRef, e, viewport)
    if (canvas === null) {
      setInteraction({ type: 'idle' })
      return
    }

    const targetBoxId = hitTestBox(elements, canvas)
    if (targetBoxId !== null && targetBoxId !== interaction.sourceBoxId) {
      const targetBox = elements.boxes.get(targetBoxId)
      const sourceBox = elements.boxes.get(interaction.sourceBoxId)
      if (targetBox !== undefined && sourceBox !== undefined && !hasArrow(elements, interaction.sourceBoxId, targetBoxId)) {
        const sourceFocusAbs = focusToAbsolute(sourceBox, interaction.sourceFocus)
        const targetFocus = computeGeometricFocus(targetBox, sourceFocusAbs)
        const arrow = createArrow(
          interaction.sourceBoxId,
          targetBoxId,
          interaction.sourceFocus,
          targetFocus,
        )
        setElements((s) => addArrow(s, arrow))
      }
    }

    setInteraction({ type: 'idle' })
  }, [containerRef, setElements, setInteraction, viewport])

  return { onArrowStart, onArrowMove, onArrowEnd } as const
}

export { useArrowDraw }
