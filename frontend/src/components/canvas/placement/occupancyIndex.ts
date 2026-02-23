import type { Rect } from '@/utils/geometry'
import {
  buildOccupancyIndex as buildOccupancyIndexGeneric,
  addToOccupancy as addToOccupancyGeneric,
  updateOccupancy as updateOccupancyGeneric,
} from '@/utils/spatial'
import type { OccupiedRect } from './types'
import { PLACEMENT } from './constants'

// Re-export functions that don't need padding injection
export { isOccupied, occupancyBounds } from '@/utils/spatial'

/**
 * Build an occupancy index using the canvas default occupancy padding.
 * Delegates to the generic `buildOccupancyIndex` from `@/utils/spatial`.
 */
const buildOccupancyIndex = (
  nodes: ReadonlyArray<{ readonly id: string; readonly rect: Rect }>,
): readonly OccupiedRect[] =>
  buildOccupancyIndexGeneric(nodes, PLACEMENT.OCCUPANCY_PAD)

/**
 * Add to occupancy using the canvas default occupancy padding.
 */
const addToOccupancy = (
  occupancy: readonly OccupiedRect[],
  id: string,
  rect: Rect,
): readonly OccupiedRect[] =>
  addToOccupancyGeneric(occupancy, id, rect, PLACEMENT.OCCUPANCY_PAD)

/**
 * Update occupancy using the canvas default occupancy padding.
 */
const updateOccupancy = (
  occupancy: readonly OccupiedRect[],
  id: string,
  newRect: Rect,
): readonly OccupiedRect[] =>
  updateOccupancyGeneric(occupancy, id, newRect, PLACEMENT.OCCUPANCY_PAD)

export { buildOccupancyIndex, addToOccupancy, updateOccupancy }
