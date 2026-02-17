import { describe, it, expect } from 'vitest'
import { Geometry } from './geometry'
import type { Point, Rect } from './geometry'

describe('Geometry', () => {
  // ── clamp ────────────────────────────────────────────────────────────

  describe('clamp', () => {
    it('returns value when within range', () => {
      expect(Geometry.clamp(5, 0, 10)).toBe(5)
    })

    it('clamps to min', () => {
      expect(Geometry.clamp(-3, 0, 10)).toBe(0)
    })

    it('clamps to max', () => {
      expect(Geometry.clamp(15, 0, 10)).toBe(10)
    })

    it('returns min when value equals min', () => {
      expect(Geometry.clamp(0, 0, 10)).toBe(0)
    })

    it('returns max when value equals max', () => {
      expect(Geometry.clamp(10, 0, 10)).toBe(10)
    })
  })

  // ── distanceBetweenPoints ────────────────────────────────────────────

  describe('distanceBetweenPoints', () => {
    it('returns 0 for identical points', () => {
      expect(Geometry.distanceBetweenPoints({ x: 5, y: 5 }, { x: 5, y: 5 })).toBe(0)
    })

    it('computes horizontal distance', () => {
      expect(Geometry.distanceBetweenPoints({ x: 0, y: 0 }, { x: 3, y: 0 })).toBe(3)
    })

    it('computes vertical distance', () => {
      expect(Geometry.distanceBetweenPoints({ x: 0, y: 0 }, { x: 0, y: 4 })).toBe(4)
    })

    it('computes diagonal distance (3-4-5 triangle)', () => {
      expect(Geometry.distanceBetweenPoints({ x: 0, y: 0 }, { x: 3, y: 4 })).toBe(5)
    })

    it('handles negative coordinates', () => {
      expect(Geometry.distanceBetweenPoints({ x: -1, y: -1 }, { x: 2, y: 3 })).toBe(5)
    })
  })

  // ── manhattanDistance ────────────────────────────────────────────────

  describe('manhattanDistance', () => {
    it('returns 0 for identical points', () => {
      expect(Geometry.manhattanDistance({ x: 5, y: 5 }, { x: 5, y: 5 })).toBe(0)
    })

    it('sums axis deltas', () => {
      expect(Geometry.manhattanDistance({ x: 0, y: 0 }, { x: 3, y: 4 })).toBe(7)
    })

    it('handles negative coordinates', () => {
      expect(Geometry.manhattanDistance({ x: -2, y: -3 }, { x: 1, y: 1 })).toBe(7)
    })
  })

  // ── rectCenter ──────────────────────────────────────────────────────

  describe('rectCenter', () => {
    it('returns center of a standard rect', () => {
      const center = Geometry.rectCenter({ x: 0, y: 0, width: 100, height: 200 })
      expect(center).toEqual({ x: 50, y: 100 })
    })

    it('handles offset rect', () => {
      const center = Geometry.rectCenter({ x: 10, y: 20, width: 100, height: 200 })
      expect(center).toEqual({ x: 60, y: 120 })
    })

    it('handles zero-size rect', () => {
      const center = Geometry.rectCenter({ x: 5, y: 5, width: 0, height: 0 })
      expect(center).toEqual({ x: 5, y: 5 })
    })
  })

  // ── rectContainsPoint ───────────────────────────────────────────────

  describe('rectContainsPoint', () => {
    const rect: Rect = { x: 10, y: 10, width: 100, height: 100 }

    it('returns true for point inside', () => {
      expect(Geometry.rectContainsPoint(rect, { x: 50, y: 50 })).toBe(true)
    })

    it('returns true for point on edge (inclusive)', () => {
      expect(Geometry.rectContainsPoint(rect, { x: 10, y: 10 })).toBe(true)
      expect(Geometry.rectContainsPoint(rect, { x: 110, y: 110 })).toBe(true)
    })

    it('returns false for point outside', () => {
      expect(Geometry.rectContainsPoint(rect, { x: 5, y: 50 })).toBe(false)
      expect(Geometry.rectContainsPoint(rect, { x: 111, y: 50 })).toBe(false)
    })
  })

  // ── nearestSide ─────────────────────────────────────────────────────

  describe('nearestSide', () => {
    const rect: Rect = { x: 0, y: 0, width: 100, height: 100 }

    it('returns top for point near top edge', () => {
      expect(Geometry.nearestSide(rect, { x: 50, y: 2 })).toBe('top')
    })

    it('returns bottom for point near bottom edge', () => {
      expect(Geometry.nearestSide(rect, { x: 50, y: 98 })).toBe('bottom')
    })

    it('returns left for point near left edge', () => {
      expect(Geometry.nearestSide(rect, { x: 2, y: 50 })).toBe('left')
    })

    it('returns right for point near right edge', () => {
      expect(Geometry.nearestSide(rect, { x: 98, y: 50 })).toBe('right')
    })

    it('returns top when point is above rect', () => {
      expect(Geometry.nearestSide(rect, { x: 50, y: -10 })).toBe('top')
    })
  })

  // ── pointAlongSide ─────────────────────────────────────────────────

  describe('pointAlongSide', () => {
    const rect: Rect = { x: 10, y: 20, width: 100, height: 200 }

    it('returns start of top side at fraction 0', () => {
      expect(Geometry.pointAlongSide(rect, 'top', 0)).toEqual({ x: 10, y: 20 })
    })

    it('returns center of top side at fraction 0.5', () => {
      expect(Geometry.pointAlongSide(rect, 'top', 0.5)).toEqual({ x: 60, y: 20 })
    })

    it('returns end of top side at fraction 1', () => {
      expect(Geometry.pointAlongSide(rect, 'top', 1)).toEqual({ x: 110, y: 20 })
    })

    it('returns 1/3 along bottom side', () => {
      const point = Geometry.pointAlongSide(rect, 'bottom', 1 / 3)
      expect(point.x).toBeCloseTo(10 + 100 / 3, 10)
      expect(point.y).toBe(220)
    })

    it('returns 1/4 along left side (top-to-bottom)', () => {
      expect(Geometry.pointAlongSide(rect, 'left', 0.25)).toEqual({ x: 10, y: 70 })
    })

    it('returns 3/4 along right side', () => {
      expect(Geometry.pointAlongSide(rect, 'right', 0.75)).toEqual({ x: 110, y: 170 })
    })
  })

  // ── sideCenter ──────────────────────────────────────────────────────

  describe('sideCenter', () => {
    const rect: Rect = { x: 10, y: 20, width: 100, height: 200 }

    it('returns midpoint of top side', () => {
      expect(Geometry.sideCenter(rect, 'top')).toEqual({ x: 60, y: 20 })
    })

    it('returns midpoint of bottom side', () => {
      expect(Geometry.sideCenter(rect, 'bottom')).toEqual({ x: 60, y: 220 })
    })

    it('returns midpoint of left side', () => {
      expect(Geometry.sideCenter(rect, 'left')).toEqual({ x: 10, y: 120 })
    })

    it('returns midpoint of right side', () => {
      expect(Geometry.sideCenter(rect, 'right')).toEqual({ x: 110, y: 120 })
    })
  })

  // ── rectsOverlap ────────────────────────────────────────────────────

  describe('rectsOverlap', () => {
    it('returns true for overlapping rects', () => {
      const a: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const b: Rect = { x: 25, y: 25, width: 50, height: 50 }
      expect(Geometry.rectsOverlap(a, b)).toBe(true)
    })

    it('returns false for touching edges (no interior overlap)', () => {
      const a: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const b: Rect = { x: 50, y: 0, width: 50, height: 50 }
      expect(Geometry.rectsOverlap(a, b)).toBe(false)
    })

    it('returns false for separated rects', () => {
      const a: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const b: Rect = { x: 100, y: 100, width: 50, height: 50 }
      expect(Geometry.rectsOverlap(a, b)).toBe(false)
    })

    it('returns true when one rect contains another', () => {
      const outer: Rect = { x: 0, y: 0, width: 100, height: 100 }
      const inner: Rect = { x: 10, y: 10, width: 20, height: 20 }
      expect(Geometry.rectsOverlap(outer, inner)).toBe(true)
    })

    it('returns false for zero-width rect touching edge', () => {
      const a: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const b: Rect = { x: 50, y: 0, width: 0, height: 50 }
      expect(Geometry.rectsOverlap(a, b)).toBe(false)
    })
  })

  // ── rectsIntersection ───────────────────────────────────────────────

  describe('rectsIntersection', () => {
    it('returns intersection rect for overlapping rects', () => {
      const a: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const b: Rect = { x: 25, y: 25, width: 50, height: 50 }
      expect(Geometry.rectsIntersection(a, b)).toEqual({ x: 25, y: 25, width: 25, height: 25 })
    })

    it('returns null for touching edges', () => {
      const a: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const b: Rect = { x: 50, y: 0, width: 50, height: 50 }
      expect(Geometry.rectsIntersection(a, b)).toBeNull()
    })

    it('returns null for separated rects', () => {
      const a: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const b: Rect = { x: 100, y: 100, width: 50, height: 50 }
      expect(Geometry.rectsIntersection(a, b)).toBeNull()
    })

    it('returns inner rect when fully contained', () => {
      const outer: Rect = { x: 0, y: 0, width: 100, height: 100 }
      const inner: Rect = { x: 10, y: 10, width: 20, height: 20 }
      expect(Geometry.rectsIntersection(outer, inner)).toEqual({ x: 10, y: 10, width: 20, height: 20 })
    })
  })

  // ── distanceBetweenRects ────────────────────────────────────────────

  describe('distanceBetweenRects', () => {
    it('returns 0 for overlapping rects', () => {
      const a: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const b: Rect = { x: 25, y: 25, width: 50, height: 50 }
      expect(Geometry.distanceBetweenRects(a, b)).toBe(0)
    })

    it('returns 0 for touching rects', () => {
      const a: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const b: Rect = { x: 50, y: 0, width: 50, height: 50 }
      expect(Geometry.distanceBetweenRects(a, b)).toBe(0)
    })

    it('returns horizontal gap for horizontally separated rects', () => {
      const a: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const b: Rect = { x: 60, y: 0, width: 50, height: 50 }
      expect(Geometry.distanceBetweenRects(a, b)).toBe(10)
    })

    it('returns diagonal distance for diagonally separated rects (3-4-5)', () => {
      const a: Rect = { x: 0, y: 0, width: 10, height: 10 }
      const b: Rect = { x: 13, y: 14, width: 10, height: 10 }
      expect(Geometry.distanceBetweenRects(a, b)).toBe(5)
    })
  })

  // ── expandRect ──────────────────────────────────────────────────────

  describe('expandRect', () => {
    it('expands by uniform padding', () => {
      const result = Geometry.expandRect({ x: 10, y: 20, width: 100, height: 200 }, 5)
      expect(result).toEqual({ x: 5, y: 15, width: 110, height: 210 })
    })

    it('handles zero padding', () => {
      const rect: Rect = { x: 10, y: 20, width: 100, height: 200 }
      expect(Geometry.expandRect(rect, 0)).toEqual(rect)
    })

    it('handles negative padding (shrink)', () => {
      const result = Geometry.expandRect({ x: 0, y: 0, width: 100, height: 100 }, -10)
      expect(result).toEqual({ x: 10, y: 10, width: 80, height: 80 })
    })
  })

  // ── boundingBox ─────────────────────────────────────────────────────

  describe('boundingBox', () => {
    it('returns zero rect for empty input', () => {
      expect(Geometry.boundingBox([])).toEqual({ x: 0, y: 0, width: 0, height: 0 })
    })

    it('returns the rect itself for single rect', () => {
      const rect: Rect = { x: 10, y: 20, width: 30, height: 40 }
      expect(Geometry.boundingBox([rect])).toEqual(rect)
    })

    it('computes bounding box of multiple rects', () => {
      const rects: Rect[] = [
        { x: 0, y: 0, width: 50, height: 50 },
        { x: 100, y: 100, width: 50, height: 50 },
        { x: 50, y: 25, width: 10, height: 10 },
      ]
      expect(Geometry.boundingBox(rects)).toEqual({ x: 0, y: 0, width: 150, height: 150 })
    })

    it('handles negative coordinates', () => {
      const rects: Rect[] = [
        { x: -50, y: -50, width: 50, height: 50 },
        { x: 50, y: 50, width: 50, height: 50 },
      ]
      expect(Geometry.boundingBox(rects)).toEqual({ x: -50, y: -50, width: 150, height: 150 })
    })
  })

  // ── snapToGrid ──────────────────────────────────────────────────────

  describe('snapToGrid', () => {
    it('snaps to nearest grid line', () => {
      expect(Geometry.snapToGrid(13, 24)).toBe(24)
      expect(Geometry.snapToGrid(11, 24)).toBe(0)
    })

    it('leaves grid-aligned values unchanged', () => {
      expect(Geometry.snapToGrid(48, 24)).toBe(48)
    })

    it('rounds midpoint up', () => {
      expect(Geometry.snapToGrid(12, 24)).toBe(24)
    })

    it('handles negative values', () => {
      expect(Geometry.snapToGrid(-13, 24)).toBe(-24)
    })
  })

  // ── snapAwayFromZero ────────────────────────────────────────────────

  describe('snapAwayFromZero', () => {
    it('snaps positive to ceil', () => {
      expect(Geometry.snapAwayFromZero(100, 24)).toBe(120) // ceil(100/24)*24 = 5*24 = 120
    })

    it('snaps negative to floor', () => {
      expect(Geometry.snapAwayFromZero(-100, 24)).toBe(-120) // floor(-100/24)*24 = -5*24 = -120
    })

    it('leaves grid-aligned positive unchanged', () => {
      expect(Geometry.snapAwayFromZero(96, 24)).toBe(96)
    })

    it('leaves grid-aligned negative unchanged', () => {
      expect(Geometry.snapAwayFromZero(-96, 24)).toBe(-96)
    })

    it('leaves zero unchanged', () => {
      expect(Geometry.snapAwayFromZero(0, 24)).toBe(0)
    })

    it('always snaps away from zero (positive just above grid line)', () => {
      expect(Geometry.snapAwayFromZero(97, 24)).toBe(120) // next grid up, not 96
    })

    it('always snaps away from zero (negative just below grid line)', () => {
      expect(Geometry.snapAwayFromZero(-97, 24)).toBe(-120) // next grid down, not -96
    })
  })

  // ── snapPointToGrid ─────────────────────────────────────────────────

  describe('snapPointToGrid', () => {
    it('snaps both coordinates', () => {
      const result = Geometry.snapPointToGrid({ x: 13, y: 37 }, 24)
      expect(result).toEqual({ x: 24, y: 48 })
    })

    it('leaves grid-aligned point unchanged', () => {
      const point: Point = { x: 48, y: 72 }
      expect(Geometry.snapPointToGrid(point, 24)).toEqual(point)
    })
  })

  // ── snapRectToGrid ──────────────────────────────────────────────────

  describe('snapRectToGrid', () => {
    it('snaps position, preserves size', () => {
      const result = Geometry.snapRectToGrid({ x: 13, y: 37, width: 100, height: 200 }, 24)
      expect(result).toEqual({ x: 24, y: 48, width: 100, height: 200 })
    })

    it('leaves grid-aligned rect unchanged', () => {
      const rect: Rect = { x: 48, y: 72, width: 100, height: 200 }
      expect(Geometry.snapRectToGrid(rect, 24)).toEqual(rect)
    })
  })
})
