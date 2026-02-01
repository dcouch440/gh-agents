import type { TreeData } from '../types'

/**
 * Assigns each node to a layer (depth) using BFS from roots.
 * For DAGs, uses longest-path layering so dependencies always appear above dependants.
 * Returns a map of nodeId → layer number (0-indexed).
 */
const assignLayers = <M>(data: TreeData<M>): Map<string, number> => {
  const layers = new Map<string, number>()

  // Initialize roots at layer 0
  const queue: string[] = [...data.rootIds]
  for (const id of queue) {
    layers.set(id, 0)
  }

  // BFS — use longest path (max of all incoming paths)
  while (queue.length > 0) {
    const id = queue.shift()!
    const node = data.nodes[id]
    if (node === undefined) continue

    const parentLayer = layers.get(id) ?? 0

    for (const childId of node.children) {
      const existingLayer = layers.get(childId)
      const newLayer = parentLayer + 1

      if (existingLayer === undefined || newLayer > existingLayer) {
        layers.set(childId, newLayer)
        queue.push(childId)
      }
    }
  }

  return layers
}

export { assignLayers }
