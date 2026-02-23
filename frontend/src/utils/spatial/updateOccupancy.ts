import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { OccupiedRect } from './types'

/**
 * Replace an existing entry's rect in the occupancy index.
 * Returns a new array. If the ID is not found, returns the original array unchanged.
 */
const updateOccupancy = (
  occupancy: readonly OccupiedRect[],
  id: string,
  newRect: Rect,
  padding: number,
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
        paddedRect: Geometry.expandRect(newRect, padding),
      })
    } else {
      result.push(occ)
    }
  }
  return found ? result : occupancy
}

export { updateOccupancy }
