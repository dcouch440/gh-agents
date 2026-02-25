// ============================================================================
// Arrow Binding — Anchor Point Computation for Arrow Endpoints
// ============================================================================
//
// Ported from Excalidraw's binding system. Key behaviors:
// - Snap-to-mid: arrows snap to edge midpoints within a threshold
// - Corner avoidance: ratio clamped to [0.1, 0.9]
// - Binding gap: arrows stop short of touching the box edge

import { Geometry } from '@/utils/geometry'
import type { Point, Side } from '@/utils/geometry'
import { BOARD } from '../constants'
import { resolveAnchor } from '../elements/bounds'
import type { AnchorPoint, BoxElement } from '../elements'

/**
 * Compute the best anchor point on a box for a cursor position.
 *
 * Finds the nearest side, then applies Excalidraw-style magnetic
 * midpoint snapping: if the cursor is within `ARROW_SNAP_THRESHOLD`
 * of the side's center, snaps to ratio 0.5.
 *
 * Ratio is clamped to [0.1, 0.9] to avoid ambiguous corner positions.
 */
const computeBindingAnchor = (box: BoxElement, cursor: Point): AnchorPoint => {
  const side = Geometry.nearestSide(box, cursor)
  const midpoint = Geometry.sideCenter(box, side)
  const distToMid = Geometry.distanceBetweenPoints(cursor, midpoint)

  if (distToMid <= BOARD.ARROW_SNAP_THRESHOLD) {
    return { side, ratio: 0.5 }
  }

  const ratio = ratioAlongSide(box, side, cursor)
  return { side, ratio: Geometry.clamp(ratio, 0.1, 0.9) }
}

/**
 * Apply the binding gap offset to a resolved anchor point.
 * Pushes the point away from the box edge by `ARROW_BINDING_GAP` px
 * in the direction normal to the anchor side.
 */
const applyBindingGap = (point: Point, side: Side): Point => {
  const gap = BOARD.ARROW_BINDING_GAP
  switch (side) {
    case 'top': return { x: point.x, y: point.y - gap }
    case 'bottom': return { x: point.x, y: point.y + gap }
    case 'left': return { x: point.x - gap, y: point.y }
    case 'right': return { x: point.x + gap, y: point.y }
  }
}

/**
 * Resolve an anchor to canvas coordinates with binding gap applied.
 */
const resolveAnchorWithGap = (box: BoxElement, anchor: AnchorPoint): Point => {
  const point = resolveAnchor(box, anchor)
  return applyBindingGap(point, anchor.side)
}

// ── Helpers ────────────────────────────────────────────────────────────────

const ratioAlongSide = (box: BoxElement, side: Side, point: Point): number => {
  switch (side) {
    case 'top':
    case 'bottom':
      return box.width > 0 ? (point.x - box.x) / box.width : 0.5
    case 'left':
    case 'right':
      return box.height > 0 ? (point.y - box.y) / box.height : 0.5
  }
}

export { applyBindingGap, computeBindingAnchor, resolveAnchorWithGap }
