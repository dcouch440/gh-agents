import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

/**
 * Returns step IDs in topological order (sources first) using Kahn's algorithm.
 * Filters out `context` and `input` execution_mode steps (not navigable).
 * Uses `display_order` as tiebreaker for same-level nodes.
 * Cycle remnants are appended by display_order for graceful degradation.
 */
const topoSortStepIds = (
  steps: readonly WorkflowStep[],
  edges: readonly WorkflowStepEdge[],
): string[] => {
  const navigable = steps.filter(
    (s) => s.execution_mode !== 'context' && s.execution_mode !== 'input',
  )
  if (navigable.length === 0) return []

  const navIds = new Set<string>()
  const orderOf = new Map<string, number>()
  for (let i = 0; i < navigable.length; i++) {
    const s = navigable[i]!
    navIds.add(s.id)
    orderOf.set(s.id, s.display_order)
  }

  const inDegree = new Map<string, number>()
  const adjacency = new Map<string, string[]>()
  for (const id of navIds) {
    inDegree.set(id, 0)
    adjacency.set(id, [])
  }

  for (let i = 0; i < edges.length; i++) {
    const e = edges[i]!
    if (!navIds.has(e.from_step_id) || !navIds.has(e.to_step_id)) continue
    adjacency.get(e.from_step_id)!.push(e.to_step_id)
    inDegree.set(e.to_step_id, (inDegree.get(e.to_step_id) ?? 0) + 1)
  }

  // Seed with zero in-degree nodes, sorted by display_order
  const queue: string[] = []
  for (const [id, deg] of inDegree) {
    if (deg === 0) queue.push(id)
  }
  queue.sort((a, b) => (orderOf.get(a) ?? 0) - (orderOf.get(b) ?? 0))

  const result: string[] = []
  const visited = new Set<string>()

  while (queue.length > 0) {
    const id = queue.shift()!
    if (visited.has(id)) continue
    visited.add(id)
    result.push(id)

    const neighbors = adjacency.get(id) ?? []
    // Collect ready neighbors, then sort by display_order before enqueuing
    const ready: string[] = []
    for (let i = 0; i < neighbors.length; i++) {
      const next = neighbors[i]!
      const d = (inDegree.get(next) ?? 1) - 1
      inDegree.set(next, d)
      if (d === 0 && !visited.has(next)) ready.push(next)
    }
    ready.sort((a, b) => (orderOf.get(a) ?? 0) - (orderOf.get(b) ?? 0))
    for (let i = 0; i < ready.length; i++) {
      queue.push(ready[i]!)
    }
  }

  // Append cycle remnants by display_order
  if (visited.size < navigable.length) {
    const remaining = navigable
      .filter((s) => !visited.has(s.id))
      .sort((a, b) => a.display_order - b.display_order)
    for (let i = 0; i < remaining.length; i++) {
      result.push(remaining[i]!.id)
    }
  }

  return result
}

export { topoSortStepIds }
