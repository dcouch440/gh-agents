import { describe, it, expect } from 'vitest'
import type { Rect } from '@/utils/geometry'
import { buildOccupancyIndex, isOccupied, addToOccupancy, occupancyBounds, updateOccupancy } from './occupancyIndex'
import { PLACEMENT } from './constants'

const makeNode = (id: string, x: number, y: number, w: number, h: number) => ({
  id,
  rect: { x, y, width: w, height: h } as Rect,
})

describe('occupancyIndex', () => {
  describe('buildOccupancyIndex', () => {
    it('returns empty array for empty input', () => {
      expect(buildOccupancyIndex([])).toHaveLength(0)
    })

    it('creates padded rects expanded by OCCUPANCY_PAD', () => {
      const nodes = [makeNode('a', 100, 200, 560, 500)]
      const index = buildOccupancyIndex(nodes)

      expect(index).toHaveLength(1)
      expect(index[0]!.id).toBe('a')
      expect(index[0]!.rect).toEqual({ x: 100, y: 200, width: 560, height: 500 })
      expect(index[0]!.paddedRect).toEqual({
        x: 100 - PLACEMENT.OCCUPANCY_PAD,
        y: 200 - PLACEMENT.OCCUPANCY_PAD,
        width: 560 + PLACEMENT.OCCUPANCY_PAD * 2,
        height: 500 + PLACEMENT.OCCUPANCY_PAD * 2,
      })
    })

    it('handles multiple nodes', () => {
      const nodes = [makeNode('a', 0, 0, 100, 100), makeNode('b', 200, 200, 100, 100)]
      const index = buildOccupancyIndex(nodes)
      expect(index).toHaveLength(2)
    })
  })

  describe('isOccupied', () => {
    it('returns false when candidate does not overlap any padded rect', () => {
      const index = buildOccupancyIndex([makeNode('a', 0, 0, 100, 100)])
      // Place candidate far away
      const candidate: Rect = { x: 500, y: 500, width: 50, height: 50 }
      expect(isOccupied(candidate, index)).toBe(false)
    })

    it('returns true when candidate overlaps a padded rect', () => {
      const index = buildOccupancyIndex([makeNode('a', 0, 0, 100, 100)])
      // Place candidate within padding zone (node ends at 100, pad extends to 124)
      const candidate: Rect = { x: 110, y: 0, width: 50, height: 50 }
      expect(isOccupied(candidate, index)).toBe(true)
    })

    it('returns false when candidate is exactly at padded boundary (touching only)', () => {
      const index = buildOccupancyIndex([makeNode('a', 0, 0, 100, 100)])
      // Padded rect extends to x = 100 + 24 = 124. Candidate starts exactly at 124 -> touching only
      const candidate: Rect = { x: 124, y: 0, width: 50, height: 50 }
      expect(isOccupied(candidate, index)).toBe(false)
    })

    it('excludes node by ID', () => {
      const index = buildOccupancyIndex([makeNode('a', 0, 0, 100, 100)])
      // Candidate overlaps, but we exclude 'a'
      const candidate: Rect = { x: 50, y: 50, width: 50, height: 50 }
      expect(isOccupied(candidate, index, 'a')).toBe(false)
    })

    it('checks all nodes in the index', () => {
      const index = buildOccupancyIndex([
        makeNode('a', 0, 0, 100, 100),
        makeNode('b', 300, 300, 100, 100),
      ])
      // Overlaps with 'b' but not 'a'
      const candidate: Rect = { x: 310, y: 310, width: 50, height: 50 }
      expect(isOccupied(candidate, index)).toBe(true)
    })
  })

  describe('addToOccupancy', () => {
    it('returns a new array with the added entry', () => {
      const original = buildOccupancyIndex([makeNode('a', 0, 0, 100, 100)])
      const updated = addToOccupancy(original, 'b', { x: 200, y: 0, width: 100, height: 100 })

      expect(updated).toHaveLength(2)
      expect(updated[1]!.id).toBe('b')
      expect(updated[1]!.rect).toEqual({ x: 200, y: 0, width: 100, height: 100 })
    })

    it('does not mutate the original array', () => {
      const original = buildOccupancyIndex([makeNode('a', 0, 0, 100, 100)])
      addToOccupancy(original, 'b', { x: 200, y: 0, width: 100, height: 100 })
      expect(original).toHaveLength(1)
    })
  })

  describe('occupancyBounds', () => {
    it('returns null for empty index', () => {
      expect(occupancyBounds([])).toBeNull()
    })

    it('returns bounding box of non-padded rects', () => {
      const index = buildOccupancyIndex([
        makeNode('a', 0, 0, 100, 100),
        makeNode('b', 200, 50, 150, 200),
      ])
      const bounds = occupancyBounds(index)
      expect(bounds).toEqual({ x: 0, y: 0, width: 350, height: 250 })
    })

    it('returns exact rect for single node', () => {
      const index = buildOccupancyIndex([makeNode('a', 50, 75, 200, 300)])
      const bounds = occupancyBounds(index)
      expect(bounds).toEqual({ x: 50, y: 75, width: 200, height: 300 })
    })
  })

  describe('updateOccupancy', () => {
    it('replaces rect and paddedRect for matching ID', () => {
      const original = buildOccupancyIndex([makeNode('a', 0, 0, 100, 100)])
      const newRect: Rect = { x: 200, y: 300, width: 150, height: 250 }
      const updated = updateOccupancy(original, 'a', newRect)

      expect(updated).toHaveLength(1)
      expect(updated[0]!.id).toBe('a')
      expect(updated[0]!.rect).toEqual(newRect)
      expect(updated[0]!.paddedRect).toEqual({
        x: 200 - PLACEMENT.OCCUPANCY_PAD,
        y: 300 - PLACEMENT.OCCUPANCY_PAD,
        width: 150 + PLACEMENT.OCCUPANCY_PAD * 2,
        height: 250 + PLACEMENT.OCCUPANCY_PAD * 2,
      })
    })

    it('returns original array unchanged when ID is not found', () => {
      const original = buildOccupancyIndex([makeNode('a', 0, 0, 100, 100)])
      const newRect: Rect = { x: 200, y: 300, width: 150, height: 250 }
      const updated = updateOccupancy(original, 'nonexistent', newRect)

      expect(updated).toBe(original) // same reference
    })

    it('does not mutate the original array', () => {
      const original = buildOccupancyIndex([
        makeNode('a', 0, 0, 100, 100),
        makeNode('b', 200, 0, 100, 100),
      ])
      const newRect: Rect = { x: 500, y: 500, width: 50, height: 50 }
      const updated = updateOccupancy(original, 'a', newRect)

      // Original should be unchanged
      expect(original[0]!.rect).toEqual({ x: 0, y: 0, width: 100, height: 100 })
      // Updated should have the new rect
      expect(updated[0]!.rect).toEqual(newRect)
      // Other entries preserved
      expect(updated[1]!.rect).toEqual({ x: 200, y: 0, width: 100, height: 100 })
    })
  })
})
