import { Geometry } from '@/utils/geometry'
import type { Point } from '@/utils/geometry'
import { isOccupied } from './isOccupied'
import { occupancyBounds } from './occupancyBounds'
import type { OccupiedRect } from './types'

/** Configuration for the free space scanning algorithm. */
type FindFreeSpaceConfig = {
  readonly gridSize: number
  readonly vGap: number
  readonly hGap: number
  readonly maxScanRows: number
  readonly maxScanCols: number
  readonly originX: number
}

/**
 * Find free space for a rectangle using a right-then-down scanning pattern.
 *
 * 1. Start at the seed point (snapped to grid).
 * 2. Scan rightward in gridSize increments.
 * 3. After maxScanCols columns, wrap to the next row (seed.y + row * vGap).
 * 4. The first non-overlapping grid-aligned position wins.
 * 5. Absolute fallback: far-right of occupancy bounds.
 */
const findFreeSpace = (
  size: { readonly width: number; readonly height: number },
  seed: Point,
  occupancy: readonly OccupiedRect[],
  config: FindFreeSpaceConfig,
): Point => {
  const snappedSeedX = Geometry.snapToGrid(seed.x, config.gridSize)
  const snappedSeedY = Geometry.snapToGrid(seed.y, config.gridSize)

  for (let row = 0; row < config.maxScanRows; row++) {
    const candidateY = snappedSeedY + row * config.vGap

    for (let col = 0; col < config.maxScanCols; col++) {
      const candidateX = snappedSeedX + col * config.gridSize
      const candidate = {
        x: candidateX,
        y: candidateY,
        width: size.width,
        height: size.height,
      }

      if (!isOccupied(candidate, occupancy)) {
        return { x: candidateX, y: candidateY }
      }
    }
  }

  // Absolute fallback: far right of existing content
  const bounds = occupancyBounds(occupancy)
  const fallbackX = bounds !== null
    ? Geometry.snapToGrid(bounds.x + bounds.width + config.hGap * 2, config.gridSize)
    : config.originX
  const fallbackY = snappedSeedY

  return { x: fallbackX, y: fallbackY }
}

export { findFreeSpace }
export type { FindFreeSpaceConfig }
