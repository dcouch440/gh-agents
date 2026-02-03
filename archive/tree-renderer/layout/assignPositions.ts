import type { LayoutOptions, Orientation, PositionedNode } from '../types'

/**
 * Assigns x/y coordinates to each node based on its layer and order within the layer.
 * Centers parents over their children (Reingold-Tilford style).
 * Swaps axes for horizontal orientation.
 */
const assignPositions = (
  layers: string[][],
  childrenMap: Map<string, string[]>,
  options: LayoutOptions,
): PositionedNode[] => {
  const { nodeWidth, nodeHeight, horizontalGap, verticalGap, orientation } = options

  // Primary axis = layers direction, secondary axis = within-layer spread
  const primaryStep = (orientation === 'vertical' ? nodeHeight : nodeWidth) + verticalGap
  const secondaryStep = (orientation === 'vertical' ? nodeWidth : nodeHeight) + horizontalGap

  // First pass: assign positions based on order within layer
  const posMap = new Map<string, { primary: number; secondary: number }>()

  for (let layerIdx = 0; layerIdx < layers.length; layerIdx++) {
    const layer = layers[layerIdx]!
    const layerWidth = layer.length * secondaryStep - horizontalGap
    const startOffset = -layerWidth / 2

    for (let nodeIdx = 0; nodeIdx < layer.length; nodeIdx++) {
      const id = layer[nodeIdx]!
      posMap.set(id, {
        primary: layerIdx * primaryStep,
        secondary: startOffset + nodeIdx * secondaryStep + (orientation === 'vertical' ? nodeWidth : nodeHeight) / 2,
      })
    }
  }

  // Second pass: center parents over children
  for (let layerIdx = layers.length - 2; layerIdx >= 0; layerIdx--) {
    const layer = layers[layerIdx]!
    for (const id of layer) {
      const children = childrenMap.get(id) ?? []
      if (children.length === 0) continue

      const childPositions = children
        .map((cid) => posMap.get(cid))
        .filter((p): p is { primary: number; secondary: number } => p !== undefined)

      if (childPositions.length === 0) continue

      const minSec = Math.min(...childPositions.map((p) => p.secondary))
      const maxSec = Math.max(...childPositions.map((p) => p.secondary))
      const center = (minSec + maxSec) / 2

      const pos = posMap.get(id)
      if (pos !== undefined) {
        pos.secondary = center
      }
    }
  }

  // Third pass: resolve overlaps within layers
  for (const layer of layers) {
    const sorted = [...layer].sort((a, b) => {
      const pa = posMap.get(a)!
      const pb = posMap.get(b)!
      return pa.secondary - pb.secondary
    })

    for (let i = 1; i < sorted.length; i++) {
      const prev = posMap.get(sorted[i - 1]!)!
      const curr = posMap.get(sorted[i]!)!
      const minGap = secondaryStep
      if (curr.secondary - prev.secondary < minGap) {
        curr.secondary = prev.secondary + minGap
      }
    }
  }

  // Convert to x/y based on orientation
  const toXY = (primary: number, secondary: number, o: Orientation): { x: number; y: number } => {
    if (o === 'vertical') return { x: secondary, y: primary }
    return { x: primary, y: secondary }
  }

  const nodes: PositionedNode[] = []
  for (const [id, pos] of posMap.entries()) {
    const { x, y } = toXY(pos.primary, pos.secondary, orientation)
    nodes.push({ id, x, y, width: nodeWidth, height: nodeHeight })
  }

  return nodes
}

export { assignPositions }
