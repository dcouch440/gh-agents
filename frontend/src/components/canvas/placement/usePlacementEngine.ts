import { useEffect, useRef } from 'react'
import { workflowStore } from '@/stores'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import { computePlacements } from './placementEngine'

// ============================================================================
// usePlacementEngine — Detect Unplaced Steps and Auto-Place Them
// ============================================================================

/**
 * Detects steps with null positions and auto-places them using the placement engine.
 *
 * Flow:
 * 1. On every steps/edges change, check for steps where position_x === null.
 * 2. If any found, call computePlacements() synchronously.
 * 3. Patch each result into the store via patchStepSilent() (no dirty flag).
 * 4. Fire-and-forget updateStep() calls to persist to the server.
 * 5. Track already-placed IDs in a ref to avoid re-processing.
 */
const usePlacementEngine = (
  steps: readonly WorkflowStep[],
  edges: readonly WorkflowStepEdge[],
): void => {
  const placedIdsRef = useRef<Set<string>>(new Set())

  useEffect(() => {
    // Find steps that need placement
    let hasUnplaced = false
    for (let i = 0; i < steps.length; i++) {
      const step = steps[i]!
      if (step.position_x === null && !placedIdsRef.current.has(step.id)) {
        hasUnplaced = true
        break
      }
    }

    if (!hasUnplaced) return

    // Compute positions for all unplaced steps
    const results = computePlacements(steps, edges)
    if (results.length === 0) return

    // Patch store silently and persist to server
    for (let i = 0; i < results.length; i++) {
      const r = results[i]!
      placedIdsRef.current.add(r.stepId)

      // Silent patch — updates store without marking dirty (no Save/Discard prompt)
      workflowStore.patchStepSilent(r.stepId, {
        position_x: r.position.x,
        position_y: r.position.y,
      })

      // Fire-and-forget server persistence
      void workflowStore.updateStep(r.stepId, {
        position_x: r.position.x,
        position_y: r.position.y,
      })
    }
  }, [steps, edges])
}

export { usePlacementEngine }
