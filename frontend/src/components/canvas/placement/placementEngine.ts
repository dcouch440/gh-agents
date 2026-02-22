import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { PlacementIntent, PlacementResult, PlacementShift, PlacementOutput } from './types'
import { classifyPlacements, resolveStepDimensions } from './topologyClassifier'
import { placePipelineNode, placeRootNode } from './pipelinePlacer'
import { placeFanOutGroup, placeConvergenceTarget } from './fanOutPlacer'
import { placeSpliceNode } from './splicePlacer'
import { findFreeSpace } from './freeSpaceFinder'
import { buildOccupancyIndex, addToOccupancy, updateOccupancy, occupancyBounds } from './occupancyIndex'
import { PLACEMENT } from './constants'

// ============================================================================
// Placement Engine — Orchestrator
// ============================================================================

/**
 * Compute positions for all unplaced steps.
 *
 * Pure function: (steps, edges, shiftableIds?) → PlacementOutput
 *
 * Algorithm:
 * 1. Build occupancy index from placed steps.
 * 2. Classify unplaced steps into topo-sorted intents.
 * 3. For each intent, route to the correct placer:
 *    - fan_out → batch siblings, placeFanOutGroup + optional convergence target
 *    - splice → placeSpliceNode with optional downstream shift
 *    - pipeline + upstream → placePipelineNode
 *    - pipeline + no upstream (root) → placeRootNode
 *    - free_space → findFreeSpace
 * 4. After each placement, update occupancy + rects so subsequent placements see it.
 * 5. Return all placements and any shifts to existing nodes.
 */
const computePlacements = (
  steps: readonly WorkflowStep[],
  edges: readonly WorkflowStepEdge[],
  shiftableIds?: ReadonlySet<string>,
): PlacementOutput => {
  // 1. Build rects for placed steps
  const rects = new Map<string, Rect>()

  for (let i = 0; i < steps.length; i++) {
    const step = steps[i]!
    if (step.position_x === null || step.position_y === null) continue

    const dims = resolveStepDimensions(step)
    rects.set(step.id, {
      x: step.position_x,
      y: step.position_y,
      width: dims.width,
      height: dims.height,
    })
  }

  // 2. Build initial occupancy from placed steps
  const placedNodes = [...rects.entries()].map(([id, rect]) => ({ id, rect }))
  let occupancy = buildOccupancyIndex(placedNodes)

  // 3. Classify unplaced steps
  const intents = classifyPlacements(steps, edges)
  if (intents.length === 0) return { placements: [], shifts: [] }

  // 4. Place each intent in topo order
  const placements: PlacementResult[] = []
  const shifts: PlacementShift[] = []
  let i = 0

  while (i < intents.length) {
    const intent = intents[i]!

    // --- Fan-out: batch siblings with the same source ---
    if (intent.strategy === 'fan_out' && intent.fanOutSourceId !== null) {
      const sourceId = intent.fanOutSourceId
      const batch = collectFanOutBatch(intents, i, sourceId)

      const sourceRect = rects.get(sourceId)
      if (sourceRect) {
        const groupResults = placeFanOutGroup(batch, sourceRect, occupancy)

        // Add all results + update occupancy
        for (let j = 0; j < groupResults.length; j++) {
          const r = groupResults[j]!
          placements.push(r)
          const placedRect: Rect = { x: r.position.x, y: r.position.y, width: batch[j]!.width, height: batch[j]!.height }
          rects.set(r.stepId, placedRect)
          occupancy = addToOccupancy(occupancy, r.stepId, placedRect)
        }

        // Convergence detection: peek at the next intent after the batch.
        // If it's pipeline and ALL its upstreams are in this fan-out group, treat as convergence target.
        const nextIdx = i + batch.length
        if (nextIdx < intents.length) {
          const nextIntent = intents[nextIdx]!
          if (nextIntent.strategy === 'pipeline' && isConvergenceTarget(nextIntent, batch, rects)) {
            const siblingRects = batch.map((b) => rects.get(b.stepId)!)
            const convergenceResult = placeConvergenceTarget(nextIntent, siblingRects, occupancy)
            placements.push(convergenceResult)
            const cRect: Rect = { x: convergenceResult.position.x, y: convergenceResult.position.y, width: nextIntent.width, height: nextIntent.height }
            rects.set(convergenceResult.stepId, cRect)
            occupancy = addToOccupancy(occupancy, convergenceResult.stepId, cRect)
            i = nextIdx + 1
            continue
          }
        }

        i += batch.length
        continue
      }

      // Source rect not found — fall through to pipeline/free_space for each
      // (shouldn't happen in practice, but handle gracefully)
    }

    // --- Splice: insert between two placed nodes ---
    if (intent.strategy === 'splice' && intent.upstreamStepId !== null && intent.spliceDownstreamId !== null) {
      const upstreamRect = rects.get(intent.upstreamStepId)
      const downstreamRect = rects.get(intent.spliceDownstreamId)

      if (upstreamRect && downstreamRect) {
        const isShiftable = shiftableIds?.has(intent.spliceDownstreamId) ?? false
        const spliceResult = placeSpliceNode(intent, upstreamRect, downstreamRect, isShiftable, occupancy)

        placements.push(spliceResult.placement)

        // Apply shift if present
        if (spliceResult.shift !== null) {
          shifts.push(spliceResult.shift)
          // Update occupancy for shifted downstream
          const shiftedRect: Rect = {
            x: downstreamRect.x + spliceResult.shift.dx,
            y: downstreamRect.y + spliceResult.shift.dy,
            width: downstreamRect.width,
            height: downstreamRect.height,
          }
          rects.set(intent.spliceDownstreamId, shiftedRect)
          occupancy = updateOccupancy(occupancy, intent.spliceDownstreamId, shiftedRect)
        }

        // Add new node to occupancy
        const placedRect: Rect = {
          x: spliceResult.placement.position.x,
          y: spliceResult.placement.position.y,
          width: intent.width,
          height: intent.height,
        }
        rects.set(intent.stepId, placedRect)
        occupancy = addToOccupancy(occupancy, intent.stepId, placedRect)

        i++
        continue
      }

      // Rects not found — fall through to pipeline/free_space
    }

    // --- Pipeline ---
    if (intent.strategy === 'pipeline' || (intent.strategy === 'fan_out') || (intent.strategy === 'splice')) {
      // fan_out/splice fell through here because source/upstream/downstream rect was missing
      let result: PlacementResult
      if (intent.upstreamStepId !== null) {
        const upstreamRect = rects.get(intent.upstreamStepId)
        if (upstreamRect) {
          result = placePipelineNode(intent, upstreamRect, occupancy)
        } else {
          result = placeRootNode(intent, occupancy)
        }
      } else {
        result = placeRootNode(intent, occupancy)
      }

      placements.push(result)
      const placedRect: Rect = { x: result.position.x, y: result.position.y, width: intent.width, height: intent.height }
      rects.set(intent.stepId, placedRect)
      occupancy = addToOccupancy(occupancy, intent.stepId, placedRect)

      i++
      continue
    }

    // --- Free space (fallback) ---
    const bounds = occupancyBounds(occupancy)
    const seed = bounds !== null
      ? {
          x: Geometry.snapToGrid(bounds.x + bounds.width + PLACEMENT.H_GAP, PLACEMENT.GRID_SIZE),
          y: Geometry.snapToGrid(bounds.y, PLACEMENT.GRID_SIZE),
        }
      : { x: PLACEMENT.ORIGIN_X, y: PLACEMENT.ORIGIN_Y }

    const result = findFreeSpace(intent, seed, occupancy)
    placements.push(result)

    const placedRect: Rect = { x: result.position.x, y: result.position.y, width: intent.width, height: intent.height }
    rects.set(intent.stepId, placedRect)
    occupancy = addToOccupancy(occupancy, intent.stepId, placedRect)

    i++
  }

  return { placements, shifts }
}

/**
 * Collect consecutive fan-out siblings with the same fanOutSourceId.
 */
const collectFanOutBatch = (
  intents: readonly PlacementIntent[],
  startIdx: number,
  sourceId: string,
): PlacementIntent[] => {
  const batch: PlacementIntent[] = []
  for (let j = startIdx; j < intents.length; j++) {
    if (intents[j]!.strategy === 'fan_out' && intents[j]!.fanOutSourceId === sourceId) {
      batch.push(intents[j]!)
    } else {
      break
    }
  }
  return batch
}

/**
 * Check if an intent is a convergence target for a fan-out group.
 * True if ALL of its upstream step IDs are in the given batch.
 */
const isConvergenceTarget = (
  intent: PlacementIntent,
  batch: readonly PlacementIntent[],
  _rects: ReadonlyMap<string, Rect>,
): boolean => {
  // A convergence target has multiple upstreams that are all fan-out siblings.
  // We check: does the intent's upstreamStepId exist in the batch?
  // AND: are there multiple fan-out siblings pointing to this intent?
  if (intent.upstreamStepId === null) return false

  const batchIds = new Set(batch.map((b) => b.stepId))

  // Check if the upstream is a batch member
  if (!batchIds.has(intent.upstreamStepId)) return false

  // Check that ALL upstream edges of this intent that point to placed/batch nodes
  // are from the batch (not from unrelated nodes).
  // We can verify this by checking: does the intent have at least 2 upstreams from the batch?
  // Since we only stored one upstreamStepId, we check downstream references instead:
  // count how many batch members list this intent in their downstreamStepIds.
  let upstreamCount = 0
  for (let i = 0; i < batch.length; i++) {
    if (batch[i]!.downstreamStepIds.includes(intent.stepId)) {
      upstreamCount++
    }
  }

  // It's a convergence target if at least 2 batch members point to it
  return upstreamCount >= 2
}

export { computePlacements }
