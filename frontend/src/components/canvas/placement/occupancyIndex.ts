import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { OccupiedRect } from './types'
import { PLACEMENT } from './constants'

// ============================================================================
// Occupancy Index — Padded Rect Collection for Collision Queries
// ============================================================================

/**
 * Build an occupancy index from existing placed nodes.
 * Each node's rect is expanded by OCCUPANCY_PAD on all sides for gap enforcement.
 */
const buildOccupancyIndex = (
  nodes: ReadonlyArray<{ readonly id: string; readonly rect: Rect }>,
): readonly OccupiedRect[] => {
  const result: OccupiedRect[] = []
  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i]!
    result.push({
      id: node.id,
      rect: node.rect,
      paddedRect: Geometry.expandRect(node.rect, PLACEMENT.OCCUPANCY_PAD),
    })
  }
  return result
}

/**
 * Test whether a candidate rect overlaps any occupied rect (using padded rects).
 * Optionally excludes a node by ID.
 */
const isOccupied = (
  candidate: Rect,
  occupancy: readonly OccupiedRect[],
  excludeId?: string,
): boolean => {
  for (let i = 0; i < occupancy.length; i++) {
    const occ = occupancy[i]!
    if (occ.id === excludeId) continue
    if (Geometry.rectsOverlap(candidate, occ.paddedRect)) return true
  }
  return false
}

/**
 * Add a newly placed node to the occupancy index.
 * Returns a new array (immutable pattern).
 */
const addToOccupancy = (
  occupancy: readonly OccupiedRect[],
  id: string,
  rect: Rect,
): readonly OccupiedRect[] => [
  ...occupancy,
  {
    id,
    rect,
    paddedRect: Geometry.expandRect(rect, PLACEMENT.OCCUPANCY_PAD),
  },
]

/**
 * Bounding box of all occupied rects (non-padded).
 * Returns null if the index is empty.
 */
const occupancyBounds = (
  occupancy: readonly OccupiedRect[],
): Rect | null => {
  if (occupancy.length === 0) return null
  const rects: Rect[] = []
  for (let i = 0; i < occupancy.length; i++) {
    rects.push(occupancy[i]!.rect)
  }
  return Geometry.boundingBox(rects)
}

/**
 * Replace an existing entry's rect in the occupancy index (for shift propagation).
 * Returns a new array. If the ID is not found, returns the original array unchanged.
 */
const updateOccupancy = (
  occupancy: readonly OccupiedRect[],
  id: string,
  newRect: Rect,
): readonly OccupiedRect[] => {
  let found = false
  const result: OccupiedRect[] = []
  for (let i = 0; i < occupancy.length; i++) {
    const occ = occupancy[i]!
    if (occ.id === id) {
      found = true
      result.push({
        id,
        rect: newRect,
        paddedRect: Geometry.expandRect(newRect, PLACEMENT.OCCUPANCY_PAD),
      })
    } else {
      result.push(occ)
    }
  }
  return found ? result : occupancy
}

export { buildOccupancyIndex, isOccupied, addToOccupancy, occupancyBounds, updateOccupancy }
