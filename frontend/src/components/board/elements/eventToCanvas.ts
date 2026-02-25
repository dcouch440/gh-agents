// ============================================================================
// Event → Canvas Coordinate Helpers
// ============================================================================

import type { Point } from '@/utils/geometry'
import type { ViewportState } from './types'
import { screenToCanvas } from './types'

/**
 * Convert a React event to canvas coordinates using the event target's bounding rect.
 * For use in handlers attached directly to the canvas wrapper div.
 */
const eventToCanvas = (
  e: { clientX: number; clientY: number; currentTarget: EventTarget | null },
  viewport: ViewportState,
): Point => {
  const wrapper = e.currentTarget as HTMLElement // safe: event handler is on a div
  const rect = wrapper.getBoundingClientRect()
  return screenToCanvas(e.clientX, e.clientY, viewport, rect)
}

/**
 * Convert a pointer event to canvas coordinates using a container ref.
 * Returns null if the container ref is not attached.
 */
const containerEventToCanvas = (
  containerRef: React.RefObject<HTMLDivElement | null>,
  e: { clientX: number; clientY: number },
  viewport: ViewportState,
): Point | null => {
  const container = containerRef.current
  if (container === null) return null
  const rect = container.getBoundingClientRect()
  return screenToCanvas(e.clientX, e.clientY, viewport, rect)
}

export { containerEventToCanvas, eventToCanvas }
