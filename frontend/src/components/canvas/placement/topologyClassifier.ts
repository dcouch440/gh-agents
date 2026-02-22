import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import { resolveVariant, VARIANT_CONFIGS } from '../CanvasNode/registry'
import type { PlacementIntent, PlacementStrategy } from './types'

// ============================================================================
// Topology Classifier — Classify Unplaced Steps into Placement Intents
// ============================================================================

/**
 * Resolve default width and height for a step based on its execution mode.
 */
const resolveStepDimensions = (
  step: WorkflowStep,
): { width: number; height: number } => {
  const variant = resolveVariant(step, new Map(), step.id)
  const config = VARIANT_CONFIGS[variant]
  return {
    width: step.width ?? config.defaultWidth,
    height: step.height ?? config.defaultHeight,
  }
}

/**
 * Classify unplaced steps into PlacementIntents sorted in topological order.
 *
 * Algorithm:
 * 1. Partition steps into placed (position_x !== null) and unplaced.
 * 2. Build upstream/downstream adjacency maps from edges.
 * 3. For each unplaced step: if any upstream neighbor is placed or was processed
 *    earlier in the topo order, assign strategy = 'pipeline'. Otherwise 'free_space'.
 * 4. Kahn's algorithm produces a topological ordering so upstream nodes are placed
 *    before their downstream neighbors.
 */
const classifyPlacements = (
  allSteps: readonly WorkflowStep[],
  edges: readonly WorkflowStepEdge[],
): readonly PlacementIntent[] => {
  // Partition into placed and unplaced
  const placedIds = new Set<string>()
  const unplacedIds = new Set<string>()
  const stepMap = new Map<string, WorkflowStep>()

  for (let i = 0; i < allSteps.length; i++) {
    const step = allSteps[i]!
    stepMap.set(step.id, step)
    if (step.position_x !== null) {
      placedIds.add(step.id)
    } else {
      unplacedIds.add(step.id)
    }
  }

  if (unplacedIds.size === 0) return []

  // Build adjacency maps (only among ALL steps, not just unplaced)
  const upstreamMap = new Map<string, string[]>()
  const downstreamMap = new Map<string, string[]>()

  for (let i = 0; i < edges.length; i++) {
    const edge = edges[i]!
    const downs = downstreamMap.get(edge.from_step_id) ?? []
    downs.push(edge.to_step_id)
    downstreamMap.set(edge.from_step_id, downs)

    const ups = upstreamMap.get(edge.to_step_id) ?? []
    ups.push(edge.from_step_id)
    upstreamMap.set(edge.to_step_id, ups)
  }

  // Kahn's algorithm on unplaced steps only
  // In-degree counts edges FROM other unplaced steps (not from placed steps)
  const inDegree = new Map<string, number>()
  for (const id of unplacedIds) {
    let count = 0
    const ups = upstreamMap.get(id) ?? []
    for (let j = 0; j < ups.length; j++) {
      if (unplacedIds.has(ups[j]!)) count++
    }
    inDegree.set(id, count)
  }

  const queue: string[] = []
  for (const [id, deg] of inDegree) {
    if (deg === 0) queue.push(id)
  }

  const sorted: string[] = []
  while (queue.length > 0) {
    const current = queue.shift()!
    sorted.push(current)

    const downs = downstreamMap.get(current) ?? []
    for (let j = 0; j < downs.length; j++) {
      const downId = downs[j]!
      if (!unplacedIds.has(downId)) continue
      const newDeg = (inDegree.get(downId) ?? 1) - 1
      inDegree.set(downId, newDeg)
      if (newDeg === 0) queue.push(downId)
    }
  }

  // Catch any unplaced steps missed by Kahn's (circular deps) — append at end
  for (const id of unplacedIds) {
    if (!sorted.includes(id)) sorted.push(id)
  }

  // Pre-compute fan-out groups: for each placed node, how many unplaced children?
  const unplacedChildrenOfPlaced = new Map<string, string[]>()
  for (const id of unplacedIds) {
    const ups = upstreamMap.get(id) ?? []
    for (let j = 0; j < ups.length; j++) {
      const upId = ups[j]!
      if (placedIds.has(upId)) {
        const children = unplacedChildrenOfPlaced.get(upId) ?? []
        children.push(id)
        unplacedChildrenOfPlaced.set(upId, children)
      }
    }
  }

  // Build intents in topo order
  // Track which IDs will be "effectively placed" as we process (for chained pipelines)
  const effectivelyPlaced = new Set(placedIds)
  const intents: PlacementIntent[] = []

  for (let i = 0; i < sorted.length; i++) {
    const stepId = sorted[i]!
    const step = stepMap.get(stepId)!
    const dims = resolveStepDimensions(step)
    const ups = upstreamMap.get(stepId) ?? []
    const downs = downstreamMap.get(stepId) ?? []

    // Find a placed (or effectively placed) upstream neighbor
    let upstreamStepId: string | null = null
    for (let j = 0; j < ups.length; j++) {
      if (effectivelyPlaced.has(ups[j]!)) {
        upstreamStepId = ups[j]!
        break
      }
    }

    // Find a placed (originally placed, not effectively placed) downstream neighbor
    let placedDownstreamId: string | null = null
    for (let j = 0; j < downs.length; j++) {
      if (placedIds.has(downs[j]!)) {
        placedDownstreamId = downs[j]!
        break
      }
    }

    // Classification priority: splice > fan_out > pipeline > free_space
    let strategy: PlacementStrategy
    let fanOutSourceId: string | null = null
    let spliceDownstreamId: string | null = null

    if (upstreamStepId !== null && placedDownstreamId !== null) {
      // Both upstream and downstream are placed → splice (insert-between)
      strategy = 'splice'
      spliceDownstreamId = placedDownstreamId
    } else if (upstreamStepId !== null) {
      // Only upstream is placed — check for fan-out
      const siblingCount = (unplacedChildrenOfPlaced.get(upstreamStepId) ?? []).length
      if (siblingCount >= 2 && placedIds.has(upstreamStepId)) {
        strategy = 'fan_out'
        fanOutSourceId = upstreamStepId
      } else {
        strategy = 'pipeline'
      }
    } else {
      strategy = 'free_space'
    }

    intents.push({
      stepId,
      width: dims.width,
      height: dims.height,
      strategy,
      upstreamStepId,
      downstreamStepIds: downs,
      fanOutSourceId,
      spliceDownstreamId,
    })

    // Mark this step as effectively placed for downstream chain processing
    effectivelyPlaced.add(stepId)
  }

  return intents
}

export { classifyPlacements, resolveStepDimensions }
