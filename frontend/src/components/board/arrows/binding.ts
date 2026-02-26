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
import type { AnchorPoint, BoxElement, FocusPoint } from '../elements'

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
  return { side, ratio: Geometry.clamp(ratio, BOARD.ANCHOR_CLAMP_MIN, BOARD.ANCHOR_CLAMP_MAX) }
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
 * Pick the side of a box that best faces toward a direction vector,
 * normalized by box dimensions to handle non-square boxes.
 */
const HORIZONTAL_BIAS = 4.0

const bestFacingSide = (dx: number, dy: number, w: number, h: number): Side => {
  // Normalize by box half-dimensions so the decision boundary
  // follows the box's diagonal, not a 45-degree line.
  // Bias toward horizontal sides — only pick top/bottom when
  // the vertical component clearly dominates.
  const halfW = w / 2 || 1
  const halfH = h / 2 || 1
  const nx = dx / halfW
  const ny = dy / halfH

  if (Math.abs(nx) * HORIZONTAL_BIAS >= Math.abs(ny)) {
    return nx > 0 ? 'right' : 'left'
  }
  return ny > 0 ? 'bottom' : 'top'
}

// ── Focus-Point Binding ───────────────────────────────────────────────────

/**
 * Convert a FocusPoint (ratios within box bounds) to absolute canvas coords.
 */
const focusToAbsolute = (box: BoxElement, focus: FocusPoint): Point => ({
  x: box.x + focus.fx * box.width,
  y: box.y + focus.fy * box.height,
})

/**
 * Ray-box intersection for an axis-aligned rectangle.
 * Given an origin inside (or on) the box and a target point,
 * finds where the ray from origin toward target crosses the box perimeter.
 */
const rayBoxIntersection = (
  box: BoxElement,
  origin: Point,
  target: Point,
): { point: Point; side: Side } => {
  const dx = target.x - origin.x
  const dy = target.y - origin.y

  // Degenerate: zero-length ray — fall back to facing side center
  if (dx === 0 && dy === 0) {
    const side: Side = 'right'
    return { point: Geometry.sideCenter(box, side), side }
  }

  let bestT = Infinity
  let bestSide: Side = 'right'
  let bestPoint: Point = Geometry.sideCenter(box, 'right')

  // Right edge: x = box.x + box.width
  if (dx !== 0) {
    const t = (box.x + box.width - origin.x) / dx
    if (t > 0 && t < bestT) {
      const y = origin.y + t * dy
      if (y >= box.y && y <= box.y + box.height) {
        bestT = t; bestSide = 'right'; bestPoint = { x: box.x + box.width, y }
      }
    }
    // Left edge: x = box.x
    const tL = (box.x - origin.x) / dx
    if (tL > 0 && tL < bestT) {
      const y = origin.y + tL * dy
      if (y >= box.y && y <= box.y + box.height) {
        bestT = tL; bestSide = 'left'; bestPoint = { x: box.x, y }
      }
    }
  }

  if (dy !== 0) {
    // Bottom edge: y = box.y + box.height
    const t = (box.y + box.height - origin.y) / dy
    if (t > 0 && t < bestT) {
      const x = origin.x + t * dx
      if (x >= box.x && x <= box.x + box.width) {
        bestT = t; bestSide = 'bottom'; bestPoint = { x, y: box.y + box.height }
      }
    }
    // Top edge: y = box.y
    const tT = (box.y - origin.y) / dy
    if (tT > 0 && tT < bestT) {
      const x = origin.x + tT * dx
      if (x >= box.x && x <= box.x + box.width) {
        bestT = tT; bestSide = 'top'; bestPoint = { x, y: box.y }
      }
    }
  }

  // If no valid intersection found (origin is outside box), use facing side
  if (bestT === Infinity) {
    const side = bestFacingSide(dx, dy, box.width, box.height)
    return { point: Geometry.sideCenter(box, side), side }
  }

  return { point: bestPoint, side: bestSide }
}

/**
 * Compute the perimeter point where a ray from focus toward target
 * exits the box, plus which side it exits from.
 */
const focusToPerimeter = (
  box: BoxElement,
  focus: FocusPoint,
  targetAbs: Point,
): { point: Point; side: Side } => {
  const focusAbs = focusToAbsolute(box, focus)
  return rayBoxIntersection(box, focusAbs, targetAbs)
}

/**
 * Convert an AnchorPoint (side + ratio) to a FocusPoint (2D ratio).
 * Used when creating arrows from edge hover, which detects side + ratio.
 */
const anchorToFocus = (anchor: AnchorPoint): FocusPoint => {
  switch (anchor.side) {
    case 'top':    return { fx: anchor.ratio, fy: 0 }
    case 'bottom': return { fx: anchor.ratio, fy: 1 }
    case 'left':   return { fx: 0, fy: anchor.ratio }
    case 'right':  return { fx: 1, fy: anchor.ratio }
  }
}

/**
 * Compute a focus point on a target box that faces the source point.
 * Replaces computeGeometricAnchor — instead of always returning ratio 0.5,
 * this projects the source direction onto the box perimeter and converts
 * to a 2D focus ratio.
 */
const computeGeometricFocus = (
  targetBox: BoxElement,
  sourceBox: BoxElement,
): FocusPoint => {
  const sx = sourceBox.x + sourceBox.width / 2
  const sy = sourceBox.y + sourceBox.height / 2
  const cx = targetBox.x + targetBox.width / 2
  const cy = targetBox.y + targetBox.height / 2
  const side = bestFacingSide(sx - cx, sy - cy, targetBox.width, targetBox.height)

  switch (side) {
    case 'left':   return { fx: 0, fy: 0.5 }
    case 'right':  return { fx: 1, fy: 0.5 }
    case 'top':    return { fx: 0.5, fy: 0 }
    case 'bottom': return { fx: 0.5, fy: 1 }
  }
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

export {
  anchorToFocus,
  applyBindingGap,
  computeBindingAnchor,
  computeGeometricFocus,
  focusToAbsolute,
  focusToPerimeter,
  ratioAlongSide,
  rayBoxIntersection,
}
