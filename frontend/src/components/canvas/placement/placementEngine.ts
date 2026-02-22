import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import type { PlacementResult } from './types'
import { classifyPlacements, resolveStepDimensions } from './topologyClassifier'
import { placePipelineNode, placeRootNode } from './pipelinePlacer'
import { findFreeSpace } from './freeSpaceFinder'
import { buildOccupancyIndex, addToOccupancy, occupancyBounds } from './occupancyIndex'
import { PLACEMENT } from './constants'

// ============================================================================
// Placement Engine — Orchestrator
// ============================================================================

/**
 * Compute positions for all unplaced steps.
 *
 * Pure function: (steps, edges) → PlacementResult[]
 *
 * Algorithm:
 * 1. Build occupancy index from placed steps.
 * 2. Classify unplaced steps into topo-sorted intents.
 * 3. For each intent, route to the correct placer:
 *    - pipeline + upstream → placePipelineNode
 *    - pipeline + no upstream (root) → placeRootNode
 *    - free_space → findFreeSpace
 * 4. After each placement, update occupancy + rects so subsequent placements see it.
 * 5. Return all results.
 */
const computePlacements = (
  steps: readonly WorkflowStep[],
  edges: readonly WorkflowStepEdge[],
): readonly PlacementResult[] => {
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
  if (intents.length === 0) return []

  // 4. Place each intent in topo order
  const results: PlacementResult[] = []

  for (let i = 0; i < intents.length; i++) {
    const intent = intents[i]!
    let result: PlacementResult

    if (intent.strategy === 'pipeline') {
      if (intent.upstreamStepId !== null) {
        const upstreamRect = rects.get(intent.upstreamStepId)
        if (upstreamRect) {
          result = placePipelineNode(intent, upstreamRect, occupancy)
        } else {
          // Upstream was classified but not yet placed — fall back to root
          result = placeRootNode(intent, occupancy)
        }
      } else {
        result = placeRootNode(intent, occupancy)
      }
    } else {
      // free_space
      const bounds = occupancyBounds(occupancy)
      const seed = bounds !== null
        ? {
            x: Geometry.snapToGrid(bounds.x + bounds.width + PLACEMENT.H_GAP, PLACEMENT.GRID_SIZE),
            y: Geometry.snapToGrid(bounds.y, PLACEMENT.GRID_SIZE),
          }
        : { x: PLACEMENT.ORIGIN_X, y: PLACEMENT.ORIGIN_Y }

      result = findFreeSpace(intent, seed, occupancy)
    }

    results.push(result)

    // 5. Add newly placed node to occupancy + rects for subsequent placements
    const placedRect: Rect = {
      x: result.position.x,
      y: result.position.y,
      width: intent.width,
      height: intent.height,
    }
    rects.set(intent.stepId, placedRect)
    occupancy = addToOccupancy(occupancy, intent.stepId, placedRect)
  }

  return results
}

export { computePlacements }
