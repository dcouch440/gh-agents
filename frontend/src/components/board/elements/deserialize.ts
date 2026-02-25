// ============================================================================
// Deserialize — Convert Saved Excalidraw JSON Back to BoardElements
// ============================================================================
//
// The GET /workflows/:id/board/elements endpoint returns the same JSON array
// that was POSTed. This module reconstructs BoardElements from that format.

import { createArrowFromSaved, createBoxFromSaved, emptyBoard } from './factory'
import { textIdForBox } from './serialize'
import type { AnchorPoint, BoardElements } from './types'

type RawElement = Record<string, unknown>

/**
 * Reconstruct BoardElements from saved Excalidraw JSON.
 *
 * Algorithm:
 * 1. Index all elements by ID
 * 2. Find rectangles with bound text → create BoxElement
 * 3. Find arrows with both bindings referencing known boxes → create ArrowElement
 * 4. Compute arrow anchors from element positions
 */
const deserializeFromExcalidraw = (elements: readonly RawElement[]): BoardElements => {
  if (elements.length === 0) return emptyBoard()

  const byId = new Map<string, RawElement>()
  for (let i = 0; i < elements.length; i++) {
    const el = elements[i]!
    const id = el['id'] as string | undefined
    if (id !== undefined) {
      byId.set(id, el)
    }
  }

  // Pass 1: Find rectangles and their bound text
  const board = emptyBoard()
  const boxes = new Map(board.boxes)
  const arrows = new Map(board.arrows)
  const boxOrder: string[] = []
  const knownBoxIds = new Set<string>()

  for (const [, el] of byId) {
    if (el['type'] !== 'rectangle') continue
    if (el['isDeleted'] === true) continue

    const id = el['id'] as string
    const text = findBoundText(el, byId)
    if (text === null) continue

    const box = createBoxFromSaved(
      id,
      el['x'] as number,
      el['y'] as number,
      el['width'] as number,
      el['height'] as number,
      text,
    )
    boxes.set(id, box)
    boxOrder.push(id)
    knownBoxIds.add(id)
  }

  // Pass 2: Find arrows between known boxes
  for (const [, el] of byId) {
    if (el['type'] !== 'arrow') continue
    if (el['isDeleted'] === true) continue

    const startBinding = el['startBinding'] as { elementId: string } | null | undefined
    const endBinding = el['endBinding'] as { elementId: string } | null | undefined

    if (startBinding === null || startBinding === undefined) continue
    if (endBinding === null || endBinding === undefined) continue

    const sourceBoxId = startBinding.elementId
    const targetBoxId = endBinding.elementId

    if (!knownBoxIds.has(sourceBoxId) || !knownBoxIds.has(targetBoxId)) continue

    const sourceBox = boxes.get(sourceBoxId)!
    const targetBox = boxes.get(targetBoxId)!

    const sourceAnchor = computeAnchorBetweenBoxes(sourceBox, targetBox)
    const targetAnchor = computeAnchorBetweenBoxes(targetBox, sourceBox)

    const arrow = createArrowFromSaved(
      el['id'] as string,
      sourceBoxId,
      targetBoxId,
      sourceAnchor,
      targetAnchor,
    )
    arrows.set(arrow.id, arrow)
  }

  return { boxes, arrows, boxOrder }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/**
 * Find the text content for a rectangle by looking at its boundElements.
 * Falls back to looking for a text element with matching containerId.
 */
const findBoundText = (rect: RawElement, byId: Map<string, RawElement>): string | null => {
  const rectId = rect['id'] as string

  // Try boundElements first
  const boundElements = rect['boundElements'] as { id: string; type: string }[] | null | undefined
  if (boundElements !== null && boundElements !== undefined && boundElements.length > 0) {
    for (let i = 0; i < boundElements.length; i++) {
      const ref = boundElements[i]!
      if (ref.type === 'text') {
        const textEl = byId.get(ref.id)
        if (textEl !== undefined) {
          const text = textEl['text'] as string | undefined
          return text ?? ''
        }
      }
    }
  }

  // Fallback: look for text with matching containerId (our deterministic ID scheme)
  const expectedTextId = textIdForBox(rectId)
  const textEl = byId.get(expectedTextId)
  if (textEl?.['type'] === 'text') {
    return (textEl['text'] as string | undefined) ?? ''
  }

  // No text found — skip this rectangle (backend also skips textless rectangles)
  return null
}

/**
 * Compute the anchor on `fromBox` that faces `toBox`.
 * Uses center-to-center direction to determine the facing side.
 */
const computeAnchorBetweenBoxes = (
  fromBox: { x: number; y: number; width: number; height: number },
  toBox: { x: number; y: number; width: number; height: number },
): AnchorPoint => {
  const fromCx = fromBox.x + fromBox.width / 2
  const fromCy = fromBox.y + fromBox.height / 2
  const toCx = toBox.x + toBox.width / 2
  const toCy = toBox.y + toBox.height / 2

  const dx = toCx - fromCx
  const dy = toCy - fromCy

  if (Math.abs(dx) > Math.abs(dy)) {
    return { side: dx > 0 ? 'right' : 'left', ratio: 0.5 }
  }
  return { side: dy > 0 ? 'bottom' : 'top', ratio: 0.5 }
}

export { deserializeFromExcalidraw }
