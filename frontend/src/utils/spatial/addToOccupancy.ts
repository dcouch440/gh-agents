import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { OccupiedRect } from './types'

/**
 * Add a newly placed rect to the occupancy index.
 * Returns a new array (immutable pattern).
 */
const addToOccupancy = (
  occupancy: readonly OccupiedRect[],
  id: string,
  rect: Rect,
  padding: number,
): readonly OccupiedRect[] => [
  ...occupancy,
  {
    id,
    rect,
    paddedRect: Geometry.expandRect(rect, padding),
  },
]

export { addToOccupancy }
