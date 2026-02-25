// ============================================================================
// Hit Testing — Point-in-Element and Anchor Zone Detection
// ============================================================================

import { Geometry } from '@/utils/geometry'
import type { Point, Rect, Side } from '@/utils/geometry'
import { computeArrowPathPoints } from '../arrows/routing'
import type { ArrowPath } from '../arrows/routing'
import { BOARD } from '../constants'
import type { BoardElements, ResizeHandle } from './types'

// ── Box Hit Testing ──────────────────────────────────────────────────────

/**
 * Returns the element ID under the given canvas point, or null.
 * Checks boxes in reverse z-order (frontmost first).
 */
const hitTest = (state: BoardElements, point: Point): string | null => {
  const boxId = hitTestBox(state, point)
  if (boxId !== null) return boxId
  return null
}

/**
 * Returns the box ID under the canvas point, or null.
 * Checks in reverse boxOrder (frontmost first).
 */
const hitTestBox = (state: BoardElements, point: Point): string | null => {
  for (let i = state.boxOrder.length - 1; i >= 0; i--) {
    const boxId = state.boxOrder[i]!
    const box = state.boxes.get(boxId)
    if (box !== undefined && Geometry.rectContainsPoint(box, point)) {
      return boxId
    }
  }
  return null
}

/**
 * Returns all element IDs whose bounding boxes intersect a selection rectangle.
 */
const hitTestRect = (state: BoardElements, rect: Rect): string[] => {
  const ids: string[] = []

  for (const [boxId, box] of state.boxes) {
    if (Geometry.rectsOverlap(rect, box)) {
      ids.push(boxId)
    }
  }

  // Include arrows whose source and target are both in the selection
  const idSet = new Set(ids)
  for (const [arrowId, arrow] of state.arrows) {
    if (idSet.has(arrow.sourceBoxId) && idSet.has(arrow.targetBoxId)) {
      ids.push(arrowId)
    }
  }

  return ids
}

/**
 * Returns a Set of all element IDs (boxes + arrows).
 */
const selectAllIds = (state: BoardElements): Set<string> => {
  const ids = new Set<string>()
  for (const id of state.boxes.keys()) ids.add(id)
  for (const id of state.arrows.keys()) ids.add(id)
  return ids
}

// ── Arrow Hit Testing ────────────────────────────────────────────────────

/**
 * Check if a point is within `threshold` px of a cubic bezier curve.
 * Samples 20 points along the curve and checks distance to each.
 */
const pointNearCubicBezier = (
  px: number,
  py: number,
  path: ArrowPath,
  threshold: number,
): boolean => {
  const steps = 20
  for (let i = 0; i <= steps; i++) {
    const t = i / steps
    const it = 1 - t
    const x = it * it * it * path.start.x + 3 * it * it * t * path.cp1.x + 3 * it * t * t * path.cp2.x + t * t * t * path.end.x
    const y = it * it * it * path.start.y + 3 * it * it * t * path.cp1.y + 3 * it * t * t * path.cp2.y + t * t * t * path.end.y
    const dx = px - x
    const dy = py - y
    if (dx * dx + dy * dy <= threshold * threshold) return true
  }
  return false
}

/**
 * Returns the arrow ID under the canvas point, or null.
 * Computes each arrow's bezier path and checks proximity.
 */
const hitTestArrow = (
  elements: BoardElements,
  point: Point,
  threshold: number,
): string | null => {
  for (const [arrowId, arrow] of elements.arrows) {
    const sourceBox = elements.boxes.get(arrow.sourceBoxId)
    const targetBox = elements.boxes.get(arrow.targetBoxId)
    if (sourceBox === undefined || targetBox === undefined) continue

    const path = computeArrowPathPoints(sourceBox, arrow.sourceAnchor, targetBox, arrow.targetAnchor)
    if (pointNearCubicBezier(point.x, point.y, path, threshold)) {
      return arrowId
    }
  }
  return null
}

// ── Edge Hover Detection ─────────────────────────────────────────────────

type EdgeHover = {
  readonly boxId: string
  readonly side: Side
  readonly ratio: number
  readonly cx: number
  readonly cy: number
}

const EDGE_HOVER_THRESHOLD = 16

/**
 * Detect which box edge the cursor is near, for arrow binding.
 * Returns the nearest edge within threshold, or null.
 */
const detectEdgeHover = (
  canvasX: number,
  canvasY: number,
  elements: BoardElements,
): EdgeHover | null => {
  for (let i = elements.boxOrder.length - 1; i >= 0; i--) {
    const boxId = elements.boxOrder[i]!
    const box = elements.boxes.get(boxId)
    if (box === undefined) continue

    const expandedLeft = box.x - EDGE_HOVER_THRESHOLD
    const expandedTop = box.y - EDGE_HOVER_THRESHOLD
    const expandedRight = box.x + box.width + EDGE_HOVER_THRESHOLD
    const expandedBottom = box.y + box.height + EDGE_HOVER_THRESHOLD

    if (canvasX < expandedLeft || canvasX > expandedRight || canvasY < expandedTop || canvasY > expandedBottom) {
      continue
    }

    const localX = canvasX - box.x
    const localY = canvasY - box.y

    const distances: { side: Side; dist: number; ratio: number }[] = [
      { side: 'top', dist: Math.abs(localY), ratio: Geometry.clamp(localX / box.width, BOARD.ANCHOR_CLAMP_MIN, BOARD.ANCHOR_CLAMP_MAX) },
      { side: 'bottom', dist: Math.abs(localY - box.height), ratio: Geometry.clamp(localX / box.width, BOARD.ANCHOR_CLAMP_MIN, BOARD.ANCHOR_CLAMP_MAX) },
      { side: 'left', dist: Math.abs(localX), ratio: Geometry.clamp(localY / box.height, BOARD.ANCHOR_CLAMP_MIN, BOARD.ANCHOR_CLAMP_MAX) },
      { side: 'right', dist: Math.abs(localX - box.width), ratio: Geometry.clamp(localY / box.height, BOARD.ANCHOR_CLAMP_MIN, BOARD.ANCHOR_CLAMP_MAX) },
    ]

    let best = distances[0]!
    for (let d = 1; d < distances.length; d++) {
      if (distances[d]!.dist < best.dist) best = distances[d]!
    }

    if (best.dist > EDGE_HOVER_THRESHOLD) continue

    let cx: number
    let cy: number
    if (best.side === 'top' || best.side === 'bottom') {
      cx = box.x + best.ratio * box.width
      cy = best.side === 'top' ? box.y : box.y + box.height
    } else {
      cx = best.side === 'left' ? box.x : box.x + box.width
      cy = box.y + best.ratio * box.height
    }

    return { boxId, side: best.side, ratio: best.ratio, cx, cy }
  }

  return null
}

// ── Resize Handle Hit Testing ────────────────────────────────────────────

const RESIZE_HIT_SIZE = 10

type ResizeHit = {
  readonly boxId: string
  readonly handle: ResizeHandle
}

const hitTestResizeHandles = (
  canvasX: number,
  canvasY: number,
  elements: BoardElements,
  selectedIds: ReadonlySet<string>,
): ResizeHit | null => {
  for (const boxId of selectedIds) {
    const box = elements.boxes.get(boxId)
    if (box === undefined) continue

    const { x, y, width: w, height: h } = box
    const half = RESIZE_HIT_SIZE / 2

    const handles: { handle: ResizeHandle; hx: number; hy: number }[] = [
      { handle: 'nw', hx: x, hy: y },
      { handle: 'ne', hx: x + w, hy: y },
      { handle: 'sw', hx: x, hy: y + h },
      { handle: 'se', hx: x + w, hy: y + h },
      { handle: 'n', hx: x + w / 2, hy: y },
      { handle: 's', hx: x + w / 2, hy: y + h },
      { handle: 'e', hx: x + w, hy: y + h / 2 },
      { handle: 'w', hx: x, hy: y + h / 2 },
    ]

    for (let i = 0; i < handles.length; i++) {
      const { handle, hx, hy } = handles[i]!
      if (Math.abs(canvasX - hx) <= half && Math.abs(canvasY - hy) <= half) {
        return { boxId, handle }
      }
    }
  }

  return null
}

// ── Cursor Mapping ───────────────────────────────────────────────────────

const RESIZE_CURSORS: Record<ResizeHandle, string> = {
  nw: 'nwse-resize', ne: 'nesw-resize', sw: 'nesw-resize', se: 'nwse-resize',
  n: 'ns-resize', s: 'ns-resize', e: 'ew-resize', w: 'ew-resize',
}

export {
  detectEdgeHover,
  hitTest,
  hitTestArrow,
  hitTestBox,
  hitTestRect,
  hitTestResizeHandles,
  pointNearCubicBezier,
  RESIZE_CURSORS,
  selectAllIds,
}
export type { EdgeHover, ResizeHit }
