import { roundCorners as roundCornersGeneric } from '@/utils/svg'
import { CONNECTOR } from '../constants'

export { parseWaypoints } from '@/utils/svg'
export type { Point } from '@/utils/geometry'

/**
 * Round corners in an SVG path using the canvas default corner radius.
 * Delegates to the generic `roundCorners` from `@/utils/svg`.
 */
const roundCorners = (path: string, radius: number = CONNECTOR.CORNER_RADIUS): string =>
  roundCornersGeneric(path, radius)

export { roundCorners }
