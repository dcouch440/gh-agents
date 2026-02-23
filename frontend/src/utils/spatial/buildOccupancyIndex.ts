import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { OccupiedRect } from './types'

/**
 * Build an occupancy index from a set of placed rectangles.
 * Each rect is expanded by `padding` on all sides for gap enforcement.
 */
const buildOccupancyIndex = (
  nodes: ReadonlyArray<{ readonly id: string; readonly rect: Rect }>,
  padding: number,
): readonly OccupiedRect[] => {
  const result: OccupiedRect[] = []
  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i]!
    result.push({
      id: node.id,
      rect: node.rect,
      paddedRect: Geometry.expandRect(node.rect, padding),
    })
  }
  return result
}

export { buildOccupancyIndex }
