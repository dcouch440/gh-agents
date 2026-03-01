// ============================================================================
// usePenDraw — Pointer Event Handler for Pen/Freedraw Tool
// ============================================================================

import { useCallback } from 'react'
import { undoStore } from '@/stores/undoStore'
import type { Point } from '@/utils/geometry'
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

type PointerLike = { readonly clientX: number; readonly clientY: number; readonly pointerType: string; readonly pressure: number }

/** Get coalesced pointer events if supported, otherwise return the single event. */
const getCoalescedOrSingle = (native: PointerEvent): readonly PointerLike[] => {
  // getCoalescedEvents is not supported in Safari — guard with 'in' check
  // so TypeScript's strict analysis doesn't flag it as unnecessary.
  if ('getCoalescedEvents' in native) {
    const coalesced = native.getCoalescedEvents()
    if (coalesced.length > 0) return coalesced
  }
  return [native]
}

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

    // Use coalesced events for high-density point capture.
    // Browsers buffer multiple pointer positions between animation frames;
    // getCoalescedEvents() gives us all of them instead of just the latest.
    // Not supported in Safari — falls back to the single event.
    const events = getCoalescedOrSingle(e.nativeEvent)

    const addedPoints: Point[] = []
    const addedPressures: number[] = []

    for (let i = 0; i < events.length; i++) {
      const evt = events[i]!
      const canvas = containerEventToCanvas(containerRef, evt, viewport)
      if (canvas === null) continue
      const pressure = evt.pointerType === 'pen' ? evt.pressure : 0.5
      addedPoints.push(canvas)
      addedPressures.push(pressure)
    }

    if (addedPoints.length === 0) return

    setInteraction({
      type: 'drawing-pen',
      points: [...interaction.points, ...addedPoints],
      pressures: [...interaction.pressures, ...addedPressures],
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
