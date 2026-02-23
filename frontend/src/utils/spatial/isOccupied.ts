import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { OccupiedRect } from './types'

/**
 * Test whether a candidate rect overlaps any occupied rect (using padded rects).
 * Optionally excludes a rect by ID.
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

export { isOccupied }
