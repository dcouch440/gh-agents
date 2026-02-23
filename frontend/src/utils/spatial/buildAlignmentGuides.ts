import type { AlignmentGuide, LayoutRect } from './types'

/**
 * Build alignment guides from a set of rectangles. Each rectangle emits 6 guides:
 * left edge, right edge, center-x (vertical axis),
 * top edge, bottom edge, center-y (horizontal axis).
 *
 * Rectangles whose ID is in `excludeIds` are skipped (typically the element being dragged).
 */
const buildAlignmentGuides = (
  nodes: readonly LayoutRect[],
  excludeIds: ReadonlySet<string>,
): readonly AlignmentGuide[] => {
  const guides: AlignmentGuide[] = []
  const n = nodes.length

  for (let i = 0; i < n; i++) {
    const node = nodes[i]!
    if (excludeIds.has(node.id)) continue

    const { x, y, width, height } = node.rect

    // Vertical guides (x-axis values)
    guides.push({ axis: 'vertical', position: x, anchorNodeId: node.id })
    guides.push({ axis: 'vertical', position: x + width, anchorNodeId: node.id })
    guides.push({ axis: 'vertical', position: x + width / 2, anchorNodeId: node.id })

    // Horizontal guides (y-axis values)
    guides.push({ axis: 'horizontal', position: y, anchorNodeId: node.id })
    guides.push({ axis: 'horizontal', position: y + height, anchorNodeId: node.id })
    guides.push({ axis: 'horizontal', position: y + height / 2, anchorNodeId: node.id })
  }

  return guides
}

export { buildAlignmentGuides }
