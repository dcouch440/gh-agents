// ============================================================================
// Serialize — Convert Internal BoardElements to Excalidraw JSON Format
// ============================================================================
//
// The backend board_serializer (classify.rs) expects Excalidraw's JSON format:
// - Rectangles with `boundElements` array containing text + arrow refs
// - Text elements with `containerId` pointing to their parent rectangle
// - Arrows with `startBinding` / `endBinding` containing `elementId`
//
// All field names are camelCase to match Rust's `#[serde(rename_all = "camelCase")]`.
// See: src/server/hub/board_serializer/types.rs

import { BOARD } from '../constants'
import type { ArrowElement, BoardElements, PenElement } from './types'

/**
 * The text element ID for a box. Deterministic so IDs are stable across
 * serialize/deserialize round trips.
 */
const textIdForBox = (boxId: string): string => `${boxId}-text`

/**
 * Convert internal BoardElements to the Excalidraw JSON array that the
 * backend expects via POST /workflows/:id/board/submit.
 *
 * Each BoxElement produces two elements:
 * 1. A rectangle with `boundElements` referencing its text + connected arrows
 * 2. A text element with `containerId` pointing to the rectangle
 *
 * Each ArrowElement produces one arrow element with `startBinding`/`endBinding`.
 */
const serializeToExcalidraw = (state: BoardElements): Record<string, unknown>[] => {
  const elements: Record<string, unknown>[] = []

  // Build reverse lookup: boxId → arrow IDs connected to it
  const boxArrowRefs = new Map<string, string[]>()
  for (const [, arrow] of state.arrows) {
    const sourceRefs = boxArrowRefs.get(arrow.sourceBoxId)
    if (sourceRefs !== undefined) {
      sourceRefs.push(arrow.id)
    } else {
      boxArrowRefs.set(arrow.sourceBoxId, [arrow.id])
    }

    const targetRefs = boxArrowRefs.get(arrow.targetBoxId)
    if (targetRefs !== undefined) {
      targetRefs.push(arrow.id)
    } else {
      boxArrowRefs.set(arrow.targetBoxId, [arrow.id])
    }
  }

  // Serialize boxes (in z-order)
  for (let i = 0; i < state.boxOrder.length; i++) {
    const boxId = state.boxOrder[i]!
    const box = state.boxes.get(boxId)
    if (box === undefined) continue

    const textId = textIdForBox(box.id)
    const arrowRefs = boxArrowRefs.get(box.id) ?? []

    // Build boundElements: text ref + arrow refs
    const boundElements: { id: string; type: string }[] = [
      { id: textId, type: 'text' },
    ]
    for (let j = 0; j < arrowRefs.length; j++) {
      boundElements.push({ id: arrowRefs[j]!, type: 'arrow' })
    }

    // Rectangle element
    elements.push({
      type: 'rectangle',
      id: box.id,
      x: box.x,
      y: box.y,
      width: box.width,
      height: box.height,
      isDeleted: false,
      boundElements,
    })

    // Text element (positioned inside the box with padding)
    elements.push({
      type: 'text',
      id: textId,
      x: box.x + BOARD.BOX_PADDING_X,
      y: box.y + BOARD.BOX_PADDING_Y,
      width: Math.max(0, box.width - BOARD.BOX_PADDING_X * 2),
      height: Math.max(0, box.height - BOARD.BOX_PADDING_Y * 2),
      isDeleted: false,
      text: box.text,
      containerId: box.id,
    })
  }

  // Serialize arrows
  for (const [, arrow] of state.arrows) {
    elements.push(serializeArrow(arrow))
  }

  // Serialize pen strokes as freedraw elements
  for (const [, pen] of state.pens) {
    elements.push(serializePen(pen))
  }

  return elements
}

const serializeArrow = (arrow: ArrowElement): Record<string, unknown> => ({
  type: 'arrow',
  id: arrow.id,
  x: 0,
  y: 0,
  width: 0,
  height: 0,
  isDeleted: false,
  startBinding: { elementId: arrow.sourceBoxId },
  endBinding: { elementId: arrow.targetBoxId },
})

/**
 * Serialize a pen stroke to Excalidraw freedraw format.
 * Points are stored as relative offsets from (x, y) base position.
 */
const serializePen = (pen: PenElement): Record<string, unknown> => {
  if (pen.points.length === 0) {
    return {
      type: 'freedraw',
      id: pen.id,
      x: 0,
      y: 0,
      isDeleted: false,
      points: [],
    }
  }

  // Compute base position (min x, min y)
  let minX = pen.points[0]!.x
  let minY = pen.points[0]!.y
  for (let i = 1; i < pen.points.length; i++) {
    const p = pen.points[i]!
    if (p.x < minX) minX = p.x
    if (p.y < minY) minY = p.y
  }

  // Convert to relative coordinates with pressure
  const points: number[][] = []
  for (let i = 0; i < pen.points.length; i++) {
    const p = pen.points[i]!
    points.push([p.x - minX, p.y - minY, pen.pressures[i] ?? 0.5])
  }

  return {
    type: 'freedraw',
    id: pen.id,
    x: minX,
    y: minY,
    isDeleted: false,
    points,
  }
}

export { serializeToExcalidraw, textIdForBox }
