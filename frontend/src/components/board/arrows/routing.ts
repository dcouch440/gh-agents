// ============================================================================
// Arrow Routing — Smooth Cubic Bezier Paths Between Boxes
// ============================================================================
//
// Computes smooth cubic bezier curves between box anchor points.
// Control points extend from each endpoint in the exit direction, creating
// natural flowing curves like Excalidraw's default arrow style.
//
// Two variants:
// - Point-based (ArrowPath) — used by Canvas 2D renderer
// - String-based (SVG path) — used by serialization and SVG fallback

import type { Point, Side } from '@/utils/geometry'
import { resolveAnchor } from '../elements/bounds'
import type { AnchorPoint, BoxElement } from '../elements'
import { applyBindingGap } from './binding'

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

// ── Point-Based API (for Canvas 2D renderer) ──────────────────────────────

/**
 * Compute arrow path points between two anchored boxes.
 *
 * The curve exits perpendicular to the source side and enters perpendicular
 * to the target side, creating a natural S-curve or C-curve.
 */
const computeArrowPathPoints = (
  sourceBox: BoxElement,
  sourceAnchor: AnchorPoint,
  targetBox: BoxElement,
  targetAnchor: AnchorPoint,
): ArrowPath => {
  const start = applyBindingGap(resolveAnchor(sourceBox, sourceAnchor), sourceAnchor.side)
  const end = applyBindingGap(resolveAnchor(targetBox, targetAnchor), targetAnchor.side)

  const dist = controlPointDistance(start, end)
  const cp1 = controlPointForSide(start, sourceAnchor.side, dist)
  const cp2 = controlPointForSide(end, targetAnchor.side, dist)

  return { start, cp1, cp2, end }
}

/**
 * Compute preview path points while the user is drawing an arrow.
 */
const computeDrawingArrowPathPoints = (
  sourceBox: BoxElement,
  sourceAnchor: AnchorPoint,
  cursorX: number,
  cursorY: number,
): ArrowPath => {
  const start = applyBindingGap(resolveAnchor(sourceBox, sourceAnchor), sourceAnchor.side)
  const end: Point = { x: cursorX, y: cursorY }

  const dist = controlPointDistance(start, end)
  const cp1 = controlPointForSide(start, sourceAnchor.side, dist)
  const approachSide = inferApproachSide(start, end)
  const cp2 = controlPointForSide(end, approachSide, dist)

  return { start, cp1, cp2, end }
}

// ── String-Based API (for SVG compatibility) ──────────────────────────────

/**
 * Compute an SVG cubic bezier path string between two anchored boxes.
 */
const computeArrowPath = (
  sourceBox: BoxElement,
  sourceAnchor: AnchorPoint,
  targetBox: BoxElement,
  targetAnchor: AnchorPoint,
): string => {
  const p = computeArrowPathPoints(sourceBox, sourceAnchor, targetBox, targetAnchor)
  return `M ${p.start.x} ${p.start.y} C ${p.cp1.x} ${p.cp1.y}, ${p.cp2.x} ${p.cp2.y}, ${p.end.x} ${p.end.y}`
}

/**
 * Compute a preview SVG path string while the user is drawing an arrow.
 */
const computeDrawingArrowPath = (
  sourceBox: BoxElement,
  sourceAnchor: AnchorPoint,
  cursorX: number,
  cursorY: number,
): string => {
  const p = computeDrawingArrowPathPoints(sourceBox, sourceAnchor, cursorX, cursorY)
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
