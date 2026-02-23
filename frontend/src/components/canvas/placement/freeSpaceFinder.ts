import type { Point } from '@/utils/geometry'
import { findFreeSpace as findFreeSpaceGeneric } from '@/utils/spatial'
import type { OccupiedRect, PlacementIntent, PlacementResult } from './types'
import { PLACEMENT } from './constants'

/**
 * Find free space for a placement intent using canvas-default placement constants.
 * Delegates to the generic `findFreeSpace` from `@/utils/spatial`.
 */
const findFreeSpace = (
  intent: PlacementIntent,
  seed: Point,
  occupancy: readonly OccupiedRect[],
): PlacementResult => {
  const position = findFreeSpaceGeneric(
    { width: intent.width, height: intent.height },
    seed,
    occupancy,
    {
      gridSize: PLACEMENT.GRID_SIZE,
      vGap: PLACEMENT.V_GAP,
      hGap: PLACEMENT.H_GAP,
      maxScanRows: PLACEMENT.MAX_SCAN_ROWS,
      maxScanCols: PLACEMENT.MAX_SCAN_COLS,
      originX: PLACEMENT.ORIGIN_X,
    },
  )

  return { stepId: intent.stepId, position }
}

export { findFreeSpace }
