// ============================================================================
// Arrow Routing — Smooth Cubic Bezier Paths Between Boxes
// ============================================================================
//
// Computes smooth cubic bezier curves between box focus points.
// The actual perimeter connection points are computed at render time via
// ray-box intersection, so arrows dynamically update when boxes move.

import { Geometry } from '@/utils/geometry'
import type { Point, Side } from '@/utils/geometry'
import type { BoxElement, FocusPoint } from '../elements'
import { applyBindingGap, bestFacingSide, focusToPerimeter } from './binding'

// ── Types ─────────────────────────────────────────────────────────────────

type ArrowPath = {
  readonly start: Point
  readonly cp1: Point
  readonly cp2: Point
  readonly end: Point
}

// ── Control Point Computation ─────────────────────────────────────────────

/**
 * Compute control point distance based on the distance between endpoints.
 * Longer distances get longer control arms for smoother curves.
 * Shorter distances get shorter arms so curves don't loop.
 */
const controlPointDistance = (start: Point, end: Point): number => {
  const dx = end.x - start.x
  const dy = end.y - start.y
  const dist = Math.sqrt(dx * dx + dy * dy)
  // Clamp between 30 and 120, scale at ~25% of distance for gentle curves
  return Math.max(30, Math.min(dist * 0.25, 120))
}

/**
 * Extend a point in the direction of the given side (outward from the box).
 */
const controlPointForSide = (point: Point, side: Side, distance: number): Point => {
  switch (side) {
    case 'top': return { x: point.x, y: point.y - distance }
    case 'bottom': return { x: point.x, y: point.y + distance }
    case 'left': return { x: point.x - distance, y: point.y }
    case 'right': return { x: point.x + distance, y: point.y }
  }
}

/** Unit normal vector pointing outward from a side. */
const sideNormal = (side: Side): Point => {
  switch (side) {
    case 'top': return { x: 0, y: -1 }
    case 'bottom': return { x: 0, y: 1 }
    case 'left': return { x: -1, y: 0 }
    case 'right': return { x: 1, y: 0 }
  }
}

/**
 * Compute a control point that blends the perpendicular exit direction (70%)
 * with the direction toward the opposite focus point (30%). This makes arrows
 * aim more naturally when boxes aren't axis-aligned.
 */
const blendedControlPoint = (
  exitPoint: Point,
  exitSide: Side,
  target: Point,
  distance: number,
): Point => {
  const perp = sideNormal(exitSide)
  const dx = target.x - exitPoint.x
  const dy = target.y - exitPoint.y
  const len = Math.sqrt(dx * dx + dy * dy) || 1
  const toTarget = { x: dx / len, y: dy / len }

  const blendX = perp.x * 0.7 + toTarget.x * 0.3
  const blendY = perp.y * 0.7 + toTarget.y * 0.3
  const blendLen = Math.sqrt(blendX * blendX + blendY * blendY) || 1

  return {
    x: exitPoint.x + (blendX / blendLen) * distance,
    y: exitPoint.y + (blendY / blendLen) * distance,
  }
}

// ── Point-Based API (for Canvas 2D renderer) ──────────────────────────────

/**
 * Compute arrow path points between two boxes.
 *
 * Exit sides and perimeter points are computed from box centers using
 * bestFacingSide (with horizontal bias), always at the side midpoint.
 * Focus points are kept in the signature for API compatibility but
 * are not used — sides are determined purely by relative box positions.
 */
const computeArrowPathPoints = (
  sourceBox: BoxElement,
  _sourceFocus: FocusPoint,
  targetBox: BoxElement,
  _targetFocus: FocusPoint,
): ArrowPath => {
  const { side: sourceSide, point: sourcePoint } = computeExitSide(sourceBox, targetBox)
  const { side: targetSide, point: targetPoint } = computeExitSide(targetBox, sourceBox)

  const start = applyBindingGap(sourcePoint, sourceSide)
  const end = applyBindingGap(targetPoint, targetSide)

  const dist = controlPointDistance(start, end)
  const targetCenter: Point = { x: targetBox.x + targetBox.width / 2, y: targetBox.y + targetBox.height / 2 }
  const sourceCenter: Point = { x: sourceBox.x + sourceBox.width / 2, y: sourceBox.y + sourceBox.height / 2 }
  const cp1 = blendedControlPoint(start, sourceSide, targetCenter, dist)
  const cp2 = blendedControlPoint(end, targetSide, sourceCenter, dist)

  return { start, cp1, cp2, end }
}

/** Compute which side of `fromBox` faces `toBox`, with the side midpoint. */
const computeExitSide = (
  fromBox: BoxElement,
  toBox: BoxElement,
): { side: Side; point: Point } => {
  const dx = (toBox.x + toBox.width / 2) - (fromBox.x + fromBox.width / 2)
  const dy = (toBox.y + toBox.height / 2) - (fromBox.y + fromBox.height / 2)
  const side = bestFacingSide(dx, dy, fromBox.width, fromBox.height)
  return { side, point: Geometry.sideCenter(fromBox, side) }
}

/**
 * Compute preview path points while the user is drawing an arrow.
 */
const computeDrawingArrowPathPoints = (
  sourceBox: BoxElement,
  sourceFocus: FocusPoint,
  cursorX: number,
  cursorY: number,
): ArrowPath => {
  const cursor: Point = { x: cursorX, y: cursorY }
  const sourceHit = focusToPerimeter(sourceBox, sourceFocus, cursor)

  const start = applyBindingGap(sourceHit.point, sourceHit.side)
  const end = cursor

  const dist = controlPointDistance(start, end)
  const cp1 = controlPointForSide(start, sourceHit.side, dist)
  const approachSide = inferApproachSide(start, end)
  const cp2 = controlPointForSide(end, approachSide, dist)

  return { start, cp1, cp2, end }
}

// ── String-Based API (for SVG compatibility) ──────────────────────────────

/**
 * Compute an SVG cubic bezier path string between two boxes.
 */
const computeArrowPath = (
  sourceBox: BoxElement,
  sourceFocus: FocusPoint,
  targetBox: BoxElement,
  targetFocus: FocusPoint,
): string => {
  const p = computeArrowPathPoints(sourceBox, sourceFocus, targetBox, targetFocus)
  return `M ${p.start.x} ${p.start.y} C ${p.cp1.x} ${p.cp1.y}, ${p.cp2.x} ${p.cp2.y}, ${p.end.x} ${p.end.y}`
}

/**
 * Compute a preview SVG path string while the user is drawing an arrow.
 */
const computeDrawingArrowPath = (
  sourceBox: BoxElement,
  sourceFocus: FocusPoint,
  cursorX: number,
  cursorY: number,
): string => {
  const p = computeDrawingArrowPathPoints(sourceBox, sourceFocus, cursorX, cursorY)
  return `M ${p.start.x} ${p.start.y} C ${p.cp1.x} ${p.cp1.y}, ${p.cp2.x} ${p.cp2.y}, ${p.end.x} ${p.end.y}`
}

// ── Helpers ────────────────────────────────────────────────────────────────

/** Infer which side the arrow approaches the endpoint from. */
const inferApproachSide = (start: Point, end: Point): Side => {
  const dx = end.x - start.x
  const dy = end.y - start.y
  if (Math.abs(dx) > Math.abs(dy)) {
    return dx > 0 ? 'left' : 'right'
  }
  return dy > 0 ? 'top' : 'bottom'
}

export {
  computeArrowPath,
  computeArrowPathPoints,
  computeDrawingArrowPath,
  computeDrawingArrowPathPoints,
}
export type { ArrowPath }
