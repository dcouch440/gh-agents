// ============================================================================
// Geometry — Static Utility Class for Canvas Layout Computations
// ============================================================================

/**
 * Pure, immutable geometry primitives and algorithms.
 * Every method accepts `readonly` inputs and never mutates.
 *
 * Follows the same V8-optimization strategy as `Collections`:
 * - Indexed `for` loops with cached length
 * - `[]` + `.push()` for PACKED element kinds
 * - Zero external dependencies
 */

type Point = {
  readonly x: number
  readonly y: number
}

type Rect = {
  readonly x: number
  readonly y: number
  readonly width: number
  readonly height: number
}

type Side = 'top' | 'right' | 'bottom' | 'left'

class Geometry {
  private constructor() {
    // Static-only — prevent instantiation
  }

  // ── Clamp ──────────────────────────────────────────────────────────────

  /** Clamp `value` to `[min, max]`. */
  static clamp(value: number, min: number, max: number): number {
    return value < min ? min : value > max ? max : value
  }

  // ── Point Operations ───────────────────────────────────────────────────

  /** Euclidean distance between two points. */
  static distanceBetweenPoints(a: Point, b: Point): number {
    const dx = a.x - b.x
    const dy = a.y - b.y
    return Math.sqrt(dx * dx + dy * dy)
  }

  /** Manhattan distance between two points (sum of axis deltas). */
  static manhattanDistance(a: Point, b: Point): number {
    return Math.abs(a.x - b.x) + Math.abs(a.y - b.y)
  }

  // ── Rect Queries ───────────────────────────────────────────────────────

  /** Center point of a rect. */
  static rectCenter(rect: Rect): Point {
    return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 }
  }

  /** Whether `point` lies inside `rect` (inclusive edges). */
  static rectContainsPoint(rect: Rect, point: Point): boolean {
    return (
      point.x >= rect.x &&
      point.x <= rect.x + rect.width &&
      point.y >= rect.y &&
      point.y <= rect.y + rect.height
    )
  }

  /** Which side of `rect` is closest to `point`. */
  static nearestSide(rect: Rect, point: Point): Side {
    const dTop = Math.abs(point.y - rect.y)
    const dBottom = Math.abs(point.y - (rect.y + rect.height))
    const dLeft = Math.abs(point.x - rect.x)
    const dRight = Math.abs(point.x - (rect.x + rect.width))

    const min = Math.min(dTop, dBottom, dLeft, dRight)
    if (min === dTop) return 'top'
    if (min === dBottom) return 'bottom'
    if (min === dLeft) return 'left'
    return 'right'
  }

  /**
   * Point at a given `fraction` (0–1) along a rect's side.
   * `0` = start of the side, `0.5` = center, `1` = end.
   * For top/bottom: left-to-right. For left/right: top-to-bottom.
   */
  static pointAlongSide(rect: Rect, side: Side, fraction: number): Point {
    switch (side) {
      case 'top':
        return { x: rect.x + rect.width * fraction, y: rect.y }
      case 'bottom':
        return { x: rect.x + rect.width * fraction, y: rect.y + rect.height }
      case 'left':
        return { x: rect.x, y: rect.y + rect.height * fraction }
      case 'right':
        return { x: rect.x + rect.width, y: rect.y + rect.height * fraction }
    }
  }

  /** Midpoint of a specific side of a rect. */
  static sideCenter(rect: Rect, side: Side): Point {
    return Geometry.pointAlongSide(rect, side, 0.5)
  }

  // ── Rect ↔ Rect Operations ────────────────────────────────────────────

  /**
   * AABB overlap test. Returns `true` if the interiors overlap
   * (touching edges only — zero-area overlap — returns `false`).
   */
  static rectsOverlap(a: Rect, b: Rect): boolean {
    return (
      a.x < b.x + b.width &&
      a.x + a.width > b.x &&
      a.y < b.y + b.height &&
      a.y + a.height > b.y
    )
  }

  /**
   * Returns the intersection rect of two rects, or `null` if they don't
   * overlap (touching edges only returns `null`).
   */
  static rectsIntersection(a: Rect, b: Rect): Rect | null {
    const x = Math.max(a.x, b.x)
    const y = Math.max(a.y, b.y)
    const right = Math.min(a.x + a.width, b.x + b.width)
    const bottom = Math.min(a.y + a.height, b.y + b.height)

    const width = right - x
    const height = bottom - y

    if (width <= 0 || height <= 0) return null
    return { x, y, width, height }
  }

  /**
   * Minimum distance between the edges of two rects.
   * Returns `0` if they overlap or touch.
   */
  static distanceBetweenRects(a: Rect, b: Rect): number {
    const dx = Math.max(0, Math.max(a.x - (b.x + b.width), b.x - (a.x + a.width)))
    const dy = Math.max(0, Math.max(a.y - (b.y + b.height), b.y - (a.y + a.height)))
    return Math.sqrt(dx * dx + dy * dy)
  }

  // ── Rect Transforms ───────────────────────────────────────────────────

  /** Expand a rect by uniform padding on all sides. */
  static expandRect(rect: Rect, padding: number): Rect {
    return {
      x: rect.x - padding,
      y: rect.y - padding,
      width: rect.width + padding * 2,
      height: rect.height + padding * 2,
    }
  }

  /** Bounding box enclosing all rects. Returns a zero rect for empty input. */
  static boundingBox(rects: readonly Rect[]): Rect {
    const n = rects.length
    if (n === 0) return { x: 0, y: 0, width: 0, height: 0 }

    let minX = rects[0]!.x
    let minY = rects[0]!.y
    let maxX = rects[0]!.x + rects[0]!.width
    let maxY = rects[0]!.y + rects[0]!.height

    for (let i = 1; i < n; i++) {
      const r = rects[i]!
      if (r.x < minX) minX = r.x
      if (r.y < minY) minY = r.y
      const rx = r.x + r.width
      const ry = r.y + r.height
      if (rx > maxX) maxX = rx
      if (ry > maxY) maxY = ry
    }

    return { x: minX, y: minY, width: maxX - minX, height: maxY - minY }
  }

  // ── Grid Snapping ─────────────────────────────────────────────────────

  /** Round a single value to the nearest grid increment. */
  static snapToGrid(value: number, gridSize: number): number {
    return Math.round(value / gridSize) * gridSize
  }

  /**
   * Snap a value to the grid, always moving away from zero.
   * Positive values snap to ceil, negative to floor.
   * Used by collision resolution to guarantee full overlap clearance.
   */
  static snapAwayFromZero(value: number, gridSize: number): number {
    if (value >= 0) return Math.ceil(value / gridSize) * gridSize
    return Math.floor(value / gridSize) * gridSize
  }

  /** Snap both coordinates of a point to the grid. */
  static snapPointToGrid(point: Point, gridSize: number): Point {
    return {
      x: Geometry.snapToGrid(point.x, gridSize),
      y: Geometry.snapToGrid(point.y, gridSize),
    }
  }

  /** Snap a rect's position to the grid (preserves width/height). */
  static snapRectToGrid(rect: Rect, gridSize: number): Rect {
    return {
      x: Geometry.snapToGrid(rect.x, gridSize),
      y: Geometry.snapToGrid(rect.y, gridSize),
      width: rect.width,
      height: rect.height,
    }
  }
}

export { Geometry }
export type { Point, Rect, Side }
