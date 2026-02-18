import { describe, it, expect } from 'vitest'
import { buildAlignmentGuides, findSnapCandidates, computeSnap } from './snapAlignment'
import { Collections } from '@/utils/collections'
import type { LayoutNode } from './types'
import type { Rect } from '@/utils/geometry'

const makeNode = (id: string, x: number, y: number, w: number, h: number): LayoutNode => ({
  id,
  kind: 'step',
  rect: { x, y, width: w, height: h },
})

describe('snapAlignment', () => {
  // ── buildAlignmentGuides ────────────────────────────────────────────

  describe('buildAlignmentGuides', () => {
    it('emits 6 guides per node (left, right, center-x, top, bottom, center-y)', () => {
      const nodes = [makeNode('a', 0, 0, 100, 100)]
      const guides = buildAlignmentGuides(nodes, new Set())
      expect(guides).toHaveLength(6)
    })

    it('excludes nodes in excludeIds', () => {
      const nodes = [makeNode('a', 0, 0, 100, 100), makeNode('b', 200, 200, 50, 50)]
      const guides = buildAlignmentGuides(nodes, new Set(['a']))
      expect(guides).toHaveLength(6) // only node 'b'
      for (const g of guides) {
        expect(g.anchorNodeId).toBe('b')
      }
    })

    it('emits correct positions for a rect at (10, 20, 100, 200)', () => {
      const nodes = [makeNode('a', 10, 20, 100, 200)]
      const guides = buildAlignmentGuides(nodes, new Set())

      const vertical = guides.filter((g) => g.axis === 'vertical')
      const horizontal = guides.filter((g) => g.axis === 'horizontal')

      expect(vertical).toHaveLength(3)
      expect(horizontal).toHaveLength(3)

      const vPositions = Collections.sortedCopy(Collections.mapBy(vertical, (g) => g.position), (a, b) => a - b)
      expect(vPositions).toEqual([10, 60, 110]) // left, center, right

      const hPositions = Collections.sortedCopy(Collections.mapBy(horizontal, (g) => g.position), (a, b) => a - b)
      expect(hPositions).toEqual([20, 120, 220]) // top, center, bottom
    })

    it('returns empty for empty input', () => {
      expect(buildAlignmentGuides([], new Set())).toHaveLength(0)
    })

    it('returns empty when all nodes excluded', () => {
      const nodes = [makeNode('a', 0, 0, 100, 100)]
      expect(buildAlignmentGuides(nodes, new Set(['a']))).toHaveLength(0)
    })
  })

  // ── findSnapCandidates ──────────────────────────────────────────────

  describe('findSnapCandidates', () => {
    const nodes = [makeNode('anchor', 100, 100, 200, 200)]
    const guides = buildAlignmentGuides(nodes, new Set())

    it('finds candidates within threshold with snapEdge', () => {
      // Dragging a rect whose left edge is at x=98 (2px from anchor left at 100)
      const dragRect: Rect = { x: 98, y: 500, width: 50, height: 50 }
      const candidates = findSnapCandidates(dragRect, guides, 5)

      const verticalCandidates = candidates.filter((c) => c.guide.axis === 'vertical')
      expect(verticalCandidates.length).toBeGreaterThan(0)
      expect(verticalCandidates[0]!.distance).toBe(2)
      expect(verticalCandidates[0]!.snapEdge).toBe('start') // left edge closest
    })

    it('returns empty when no guides within threshold', () => {
      const dragRect: Rect = { x: 500, y: 500, width: 50, height: 50 }
      const candidates = findSnapCandidates(dragRect, guides, 5)
      expect(candidates).toHaveLength(0)
    })

    it('sorts by distance ascending', () => {
      // Place drag rect so multiple guides are in range
      const dragRect: Rect = { x: 99, y: 99, width: 50, height: 50 }
      const candidates = findSnapCandidates(dragRect, guides, 10)

      for (let i = 1; i < candidates.length; i++) {
        expect(candidates[i]!.distance).toBeGreaterThanOrEqual(candidates[i - 1]!.distance)
      }
    })

    it('checks right edge and center-x too', () => {
      // Drag rect whose right edge (x + width = 300) aligns with anchor right (300)
      const dragRect: Rect = { x: 250, y: 500, width: 50, height: 50 }
      const candidates = findSnapCandidates(dragRect, guides, 2)
      const verticals = candidates.filter((c) => c.guide.axis === 'vertical')
      expect(verticals.length).toBeGreaterThan(0)
      const rightSnap = verticals.find((c) => c.guide.position === 300)
      expect(rightSnap).toBeDefined()
      expect(rightSnap!.snapEdge).toBe('end') // right edge closest
    })
  })

  // ── computeSnap ─────────────────────────────────────────────────────

  describe('computeSnap', () => {
    it('returns original position when no candidates', () => {
      const dragRect: Rect = { x: 50, y: 60, width: 100, height: 100 }
      const result = computeSnap(dragRect, [])
      expect(result.snappedX).toBe(50)
      expect(result.snappedY).toBe(60)
      expect(result.activeGuides).toHaveLength(0)
    })

    it('snaps left edge to vertical guide', () => {
      const dragRect: Rect = { x: 98, y: 200, width: 100, height: 100 }
      // Guide at x=100 — drag left edge is at 98 (distance 2)
      const candidates = findSnapCandidates(
        dragRect,
        [{ axis: 'vertical', position: 100, anchorNodeId: 'a' }],
        5,
      )
      const result = computeSnap(dragRect, candidates)
      expect(result.snappedX).toBe(100) // snapped left edge to 100
      expect(result.activeGuides).toHaveLength(1)
    })

    it('snaps right edge to vertical guide', () => {
      const dragRect: Rect = { x: 0, y: 200, width: 100, height: 100 }
      // Guide at x=102 — drag right edge is at 100 (distance 2)
      const candidates = findSnapCandidates(
        dragRect,
        [{ axis: 'vertical', position: 102, anchorNodeId: 'a' }],
        5,
      )
      const result = computeSnap(dragRect, candidates)
      expect(result.snappedX).toBe(2) // 102 - 100 width = 2
      expect(result.activeGuides).toHaveLength(1)
    })

    it('snaps top edge to horizontal guide', () => {
      const dragRect: Rect = { x: 200, y: 48, width: 100, height: 100 }
      // Guide at y=50 — drag top is at 48 (distance 2)
      const candidates = findSnapCandidates(
        dragRect,
        [{ axis: 'horizontal', position: 50, anchorNodeId: 'a' }],
        5,
      )
      const result = computeSnap(dragRect, candidates)
      expect(result.snappedY).toBe(50)
      expect(result.activeGuides).toHaveLength(1)
    })

    it('snaps independently on both axes', () => {
      const dragRect: Rect = { x: 98, y: 48, width: 100, height: 100 }
      const guides = [
        { axis: 'vertical' as const, position: 100, anchorNodeId: 'a' },
        { axis: 'horizontal' as const, position: 50, anchorNodeId: 'b' },
      ]
      const candidates = findSnapCandidates(dragRect, guides, 5)
      const result = computeSnap(dragRect, candidates)
      expect(result.snappedX).toBe(100)
      expect(result.snappedY).toBe(50)
      expect(result.activeGuides).toHaveLength(2)
    })

    it('picks closest candidate on each axis', () => {
      const dragRect: Rect = { x: 98, y: 200, width: 100, height: 100 }
      // Two vertical guides: one at 100 (dist 2), one at 110 (dist 12)
      const guides = [
        { axis: 'vertical' as const, position: 100, anchorNodeId: 'a' },
        { axis: 'vertical' as const, position: 110, anchorNodeId: 'b' },
      ]
      const candidates = findSnapCandidates(dragRect, guides, 15)
      const result = computeSnap(dragRect, candidates)
      expect(result.snappedX).toBe(100) // picks closest
      expect(result.activeGuides).toHaveLength(1)
      expect(result.activeGuides[0]!.anchorNodeId).toBe('a')
    })
  })
})
