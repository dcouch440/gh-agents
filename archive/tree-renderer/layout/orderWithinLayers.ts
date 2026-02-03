import type { TreeData } from '../types'

/**
 * Orders nodes within each layer to minimize edge crossings.
 * Uses a barycenter heuristic with 2 passes (down then up).
 * Returns layers as an array of arrays of node IDs.
 */
const orderWithinLayers = <M>(
  data: TreeData<M>,
  layerMap: Map<string, number>,
): string[][] => {
  // Build initial layer arrays
  const maxLayer = Math.max(0, ...layerMap.values())
  const layers: string[][] = Array.from({ length: maxLayer + 1 }, () => [])

  for (const [id, layer] of layerMap.entries()) {
    layers[layer]!.push(id)
  }

  // Initial order: roots in rootIds order, rest by insertion
  layers[0] = data.rootIds.filter((id) => layerMap.get(id) === 0)

  // Build parent→child and child→parent index maps
  const childrenOf = new Map<string, string[]>()
  const parentsOf = new Map<string, string[]>()

  for (const [id, node] of Object.entries(data.nodes)) {
    childrenOf.set(id, node.children)
    for (const childId of node.children) {
      const existing = parentsOf.get(childId) ?? []
      existing.push(id)
      parentsOf.set(childId, existing)
    }
  }

  const indexOf = (layer: string[], id: string): number => {
    const idx = layer.indexOf(id)
    return idx === -1 ? 0 : idx
  }

  // Barycenter: average position of connected nodes in adjacent layer
  const barycenter = (nodeId: string, adjacentLayer: string[], getConnected: (id: string) => string[]): number => {
    const connected = getConnected(nodeId).filter((cid) => adjacentLayer.includes(cid))
    if (connected.length === 0) return indexOf(adjacentLayer, nodeId)
    const sum = connected.reduce((s, cid) => s + indexOf(adjacentLayer, cid), 0)
    return sum / connected.length
  }

  // Forward pass (top-down): order children by parent positions
  for (let i = 1; i <= maxLayer; i++) {
    const prevLayer = layers[i - 1]!
    const currentLayer = layers[i]!
    currentLayer.sort((a, b) => {
      const ba = barycenter(a, prevLayer, (id) => parentsOf.get(id) ?? [])
      const bb = barycenter(b, prevLayer, (id) => parentsOf.get(id) ?? [])
      return ba - bb
    })
  }

  // Backward pass (bottom-up): order parents by child positions
  for (let i = maxLayer - 1; i >= 0; i--) {
    const nextLayer = layers[i + 1]!
    const currentLayer = layers[i]!
    currentLayer.sort((a, b) => {
      const ba = barycenter(a, nextLayer, (id) => childrenOf.get(id) ?? [])
      const bb = barycenter(b, nextLayer, (id) => childrenOf.get(id) ?? [])
      return ba - bb
    })
  }

  return layers
}

export { orderWithinLayers }
