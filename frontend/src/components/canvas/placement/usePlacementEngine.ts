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
 * 3. Patch each placement into the store via patchStepSilent() (no dirty flag).
 * 4. Apply any shifts to existing nodes (splice nudging).
 * 5. Fire-and-forget updateStep() calls to persist to the server.
 * 6. Track already-placed IDs in a ref — these become "shiftable" for future splices.
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

    // Compute positions for all unplaced steps (pass shiftable IDs for splice)
    const output = computePlacements(steps, edges, placedIdsRef.current)
    if (output.placements.length === 0 && output.shifts.length === 0) return

    // Patch new placements into store silently and persist to server
    for (let i = 0; i < output.placements.length; i++) {
      const r = output.placements[i]!
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

    // Apply shifts to existing nodes (splice nudging)
    for (let i = 0; i < output.shifts.length; i++) {
      const s = output.shifts[i]!
      const step = steps.find((st) => st.id === s.stepId)
      if (!step) continue
      if (step.position_x === null || step.position_y === null) continue

      const newX = step.position_x + s.dx
      const newY = step.position_y + s.dy

      workflowStore.patchStepSilent(s.stepId, {
        position_x: newX,
        position_y: newY,
      })

      void workflowStore.updateStep(s.stepId, {
        position_x: newX,
        position_y: newY,
      })
    }
  }, [steps, edges])
}

export { usePlacementEngine }
