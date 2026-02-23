import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { OccupiedRect } from './types'

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

export { occupancyBounds }
