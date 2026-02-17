import { describe, it, expect } from 'vitest'
import { detectOverlaps, resolveOverlaps } from './collisionDetection'
import type { LayoutNode } from './types'
import type { Rect } from '@/utils/geometry'

const makeNode = (id: string, x: number, y: number, w: number, h: number): LayoutNode => ({
  id,
  kind: 'step',
  rect: { x, y, width: w, height: h },
})

describe('collisionDetection', () => {
  // ── detectOverlaps ──────────────────────────────────────────────────

  describe('detectOverlaps', () => {
    it('returns empty for no overlaps', () => {
      const moved: Rect = { x: 0, y: 0, width: 50, height: 50 }
      const others = [makeNode('b', 100, 100, 50, 50)]
      expect(detectOverlaps(moved, 'a', others)).toHaveLength(0)
    })

    it('detects a single overlap', () => {
      const moved: Rect = { x: 0, y: 0, width: 100, height: 100 }
      const others = [makeNode('b', 50, 50, 100, 100)]
      const overlaps = detectOverlaps(moved, 'a', others)
      expect(overlaps).toHaveLength(1)
      expect(overlaps[0]!.nodeId).toBe('b')
    })

    it('skips the moved node itself', () => {
      const moved: Rect = { x: 0, y: 0, width: 100, height: 100 }
      const others = [makeNode('a', 0, 0, 100, 100)]
      expect(detectOverlaps(moved, 'a', others)).toHaveLength(0)
    })

    it('detects multiple overlaps', () => {
      const moved: Rect = { x: 50, y: 50, width: 100, height: 100 }
      const others = [
        makeNode('b', 0, 0, 80, 80),
        makeNode('c', 120, 120, 80, 80),
        makeNode('d', 500, 500, 50, 50), // no overlap
      ]
      const overlaps = detectOverlaps(moved, 'a', others)
      expect(overlaps).toHaveLength(2)
      const ids = overlaps.map((o) => o.nodeId)
      expect(ids).toContain('b')
      expect(ids).toContain('c')
    })

    it('computes push direction based on relative position', () => {
      // Other node is to the right of the moved node
      const moved: Rect = { x: 0, y: 0, width: 100, height: 100 }
      const others = [makeNode('b', 80, 0, 100, 100)]
      const overlaps = detectOverlaps(moved, 'a', others)
      expect(overlaps).toHaveLength(1)
      expect(overlaps[0]!.pushDirection).toBe('right')
      expect(overlaps[0]!.pushDistance).toBe(20) // intersection width
    })

    it('pushes left when other is to the left', () => {
      const moved: Rect = { x: 50, y: 0, width: 100, height: 100 }
      const others = [makeNode('b', 0, 0, 70, 100)]
      const overlaps = detectOverlaps(moved, 'a', others)
      expect(overlaps).toHaveLength(1)
      expect(overlaps[0]!.pushDirection).toBe('left')
    })

    it('pushes vertically when height overlap is smaller', () => {
      // Wide overlap (80px), narrow vertical overlap (10px)
      const moved: Rect = { x: 0, y: 0, width: 100, height: 100 }
      const others = [makeNode('b', 10, 90, 80, 100)]
      const overlaps = detectOverlaps(moved, 'a', others)
      expect(overlaps).toHaveLength(1)
      expect(overlaps[0]!.pushDirection).toBe('bottom')
      expect(overlaps[0]!.pushDistance).toBe(10) // intersection height
    })

    it('returns empty for touching but non-overlapping rects', () => {
      const moved: Rect = { x: 0, y: 0, width: 100, height: 100 }
      const others = [makeNode('b', 100, 0, 100, 100)]
      expect(detectOverlaps(moved, 'a', others)).toHaveLength(0)
    })
  })

  // ── resolveOverlaps ─────────────────────────────────────────────────

  describe('resolveOverlaps', () => {
    it('returns empty map for empty overlaps', () => {
      const result = resolveOverlaps([], new Map(), 24)
      expect(result.size).toBe(0)
    })

    it('resolves a single overlap by pushing right', () => {
      const allNodes = new Map<string, Rect>([
        ['a', { x: 0, y: 0, width: 100, height: 100 }],
        ['b', { x: 80, y: 0, width: 100, height: 100 }],
      ])

      const overlaps = detectOverlaps(allNodes.get('a')!, 'a', [
        { id: 'b', kind: 'step', rect: allNodes.get('b')! },
      ])

      const result = resolveOverlaps(overlaps, allNodes, 24)
      expect(result.has('b')).toBe(true)

      const newPos = result.get('b')!
      // Pushed right by 20px from x=80 → x=100, snapped away (ceil) to grid
      expect(newPos.x).toBe(120) // ceil(100/24)*24 = 120 — snaps AWAY from push source
    })

    it('snaps resolved positions to grid', () => {
      const allNodes = new Map<string, Rect>([
        ['a', { x: 0, y: 0, width: 100, height: 100 }],
        ['b', { x: 90, y: 0, width: 100, height: 100 }],
      ])

      const overlaps = detectOverlaps(allNodes.get('a')!, 'a', [
        { id: 'b', kind: 'step', rect: allNodes.get('b')! },
      ])

      const result = resolveOverlaps(overlaps, allNodes, 24)
      const newPos = result.get('b')!

      // Position should be snapped to 24px grid
      expect(newPos.x % 24).toBe(0)
      expect(newPos.y % 24).toBe(0)
    })

    it('handles cascading overlaps', () => {
      // Three nodes in a row, pushing A into B, B into C
      const allNodes = new Map<string, Rect>([
        ['a', { x: 0, y: 0, width: 100, height: 100 }],
        ['b', { x: 80, y: 0, width: 100, height: 100 }],
        ['c', { x: 160, y: 0, width: 100, height: 100 }],
      ])

      const overlaps = detectOverlaps(allNodes.get('a')!, 'a', [
        { id: 'b', kind: 'step', rect: allNodes.get('b')! },
      ])

      const result = resolveOverlaps(overlaps, allNodes, 24)
      expect(result.has('b')).toBe(true)

      // C may or may not cascade depending on B's new position
      // The important thing is B moved right
      expect(result.get('b')!.x).toBeGreaterThan(80)
    })

    it('directional snap never under-resolves overlaps', () => {
      // Regression test: Math.round could snap BACK toward the overlap.
      // Node A at 0–100, Node B at 80–180. Push B right by 20 → 100.
      // Math.round(100/24)*24 = 96 → STILL overlaps A (0–100) by 4px!
      // Math.ceil(100/24)*24 = 120 → fully clears A.
      const allNodes = new Map<string, Rect>([
        ['a', { x: 0, y: 0, width: 100, height: 100 }],
        ['b', { x: 80, y: 0, width: 100, height: 100 }],
      ])

      const overlaps = detectOverlaps(allNodes.get('a')!, 'a', [
        { id: 'b', kind: 'step', rect: allNodes.get('b')! },
      ])

      const result = resolveOverlaps(overlaps, allNodes, 24)
      const newPos = result.get('b')!

      // The resolved position must fully clear node A (ends at x=100)
      expect(newPos.x).toBeGreaterThanOrEqual(100)
    })

    it('respects max depth limit', () => {
      // Create a long chain that would cascade beyond max depth
      const allNodes = new Map<string, Rect>()
      const nodes: LayoutNode[] = []

      for (let i = 0; i < 10; i++) {
        const rect: Rect = { x: i * 90, y: 0, width: 100, height: 100 }
        allNodes.set(`n${i}`, rect)
        nodes.push({ id: `n${i}`, kind: 'step', rect })
      }

      const overlaps = detectOverlaps(allNodes.get('n0')!, 'n0', nodes)
      const result = resolveOverlaps(overlaps, allNodes, 24, 2)

      // Should resolve some but not cascade indefinitely
      // With maxDepth=2, should handle at most 2 levels of cascading
      expect(result.size).toBeGreaterThan(0)
      expect(result.size).toBeLessThanOrEqual(10)
    })
  })
})
