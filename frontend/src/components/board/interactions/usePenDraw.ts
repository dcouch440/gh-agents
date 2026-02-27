// ============================================================================
// usePenDraw — Pointer Event Handler for Pen/Freedraw Tool
// ============================================================================

import { useCallback } from 'react'
import { undoStore } from '@/stores/undoStore'
import { containerEventToCanvas } from '../elements'
import type { InteractionMode, ViewportState } from '../elements'
import { addPen, createPen } from '../elements'
import type { SetElements, SetInteraction } from './types'

type PenDrawActions = {
  readonly onPenStart: (e: React.PointerEvent) => void
  readonly onPenMove: (e: React.PointerEvent, interaction: InteractionMode) => void
  readonly onPenEnd: (interaction: InteractionMode) => void
}

const MIN_POINTS_FOR_STROKE = 3

const usePenDraw = (
  setElements: SetElements,
  setInteraction: SetInteraction,
  viewport: ViewportState,
  containerRef: React.RefObject<HTMLDivElement | null>,
): PenDrawActions => {

  const onPenStart = useCallback((e: React.PointerEvent) => {
    const canvas = containerEventToCanvas(containerRef, e, viewport)
    if (canvas === null) return

    const pressure = e.pointerType === 'pen' ? e.pressure : 0.5
    setInteraction({
      type: 'drawing-pen',
      points: [canvas],
      pressures: [pressure],
    })
  }, [containerRef, setInteraction, viewport])

  const onPenMove = useCallback((e: React.PointerEvent, interaction: InteractionMode) => {
    if (interaction.type !== 'drawing-pen') return

    const canvas = containerEventToCanvas(containerRef, e, viewport)
    if (canvas === null) return

    const pressure = e.pointerType === 'pen' ? e.pressure : 0.5
    setInteraction({
      type: 'drawing-pen',
      points: [...interaction.points, canvas],
      pressures: [...interaction.pressures, pressure],
    })
  }, [containerRef, setInteraction, viewport])

  const onPenEnd = useCallback((interaction: InteractionMode) => {
    if (interaction.type !== 'drawing-pen') return

    if (interaction.points.length >= MIN_POINTS_FOR_STROKE) {
      undoStore.push('draw-pen')
      const pen = createPen(interaction.points, interaction.pressures)
      setElements((s) => addPen(s, pen))
    }

    setInteraction({ type: 'idle' })
  }, [setElements, setInteraction])

  return { onPenStart, onPenMove, onPenEnd }
}

export { usePenDraw }
