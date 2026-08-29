import { useMemo } from 'react'
import { useTheme } from '@mui/material/styles'
import { useStore, boardStore, workflowLiveStore, workflowExecutionStore } from '@/stores'
import { Collections } from '@/utils/collections'
import { resolveNodeStatus } from '@/utils/resolveNodeStatus'
import { resolveStatusRing } from '@/utils/statusRing'
import type { StatusRing } from '@/utils/statusRing'
import { BOARD_RING } from '../constants'

/**
 * A status ring per board box, keyed by element id.
 *
 * A step's box can be found under either of two ids, because the board has two
 * element regimes and switches between them at runtime:
 *
 * - Boxes the user drew keep their client-generated ids. `POST /board` returns
 *   the element→step pairing, which `boardStore.elementStepMap` holds and
 *   `canvas_snapshots.last_response_json` persists across a refresh.
 * - `workflow_agent::sync::sync_canvas_elements` rebuilds the board from the
 *   steps whenever the manager agent adds, removes or edits a node, and the
 *   elements it writes use `step.id` as the element id. It broadcasts
 *   `board_elements_updated`, and `refreshBoardElements` swaps the board over.
 *
 * Nothing clears `elementStepMap` when that swap happens, so the map keeps
 * pointing at client ids that no longer exist on the board. Keying a ring only
 * by the mapped id therefore dropped every ring the moment the manager agent
 * touched the workflow — a node mid-design went blue, then back to a bare
 * outline, and stayed that way until a page refresh (which rebuilds the map
 * empty, because the same rebuild nulls `last_response_json`).
 *
 * Registering the ring under both ids is what fixes that, and it costs nothing:
 * the two are the same string in the rebuilt regime, and in the drawn regime
 * only one of them is ever on the board. Neither id can be trusted alone, so
 * neither is asked to be.
 *
 * Status comes from `resolveNodeStatus`, the same resolver the sidebar uses,
 * so a box and its sidebar row can never disagree.
 *
 * Deliberately excludes the pulse phase: this map changes only when run state
 * changes, while the pulse changes every frame. Keeping them apart means the
 * animation never rebuilds the map.
 */
const useBoardStatusRings = (zoom: number): ReadonlyMap<string, StatusRing> => {
  const theme = useTheme()
  const elementStepMap = useStore(boardStore.store, (s) => s.elementStepMap)
  const baselineByStep = useStore(workflowLiveStore.store, workflowLiveStore.selectBaselineByStep)
  const dispatches = useStore(workflowLiveStore.store, workflowLiveStore.selectDispatches)
  const stepStates = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStepStates)

  // Zoomed out, a board full of breathing rings is noise, not information.
  const animated = zoom >= BOARD_RING.ANIMATE_MIN_ZOOM

  return useMemo(() => {
    const rings = new Map<string, StatusRing>()
    const dispatchByStep = Collections.keyBy(dispatches, (d) => d.stepId)

    // step id -> the client element id it was drawn as, when the board is still
    // showing user-drawn boxes. Absent once the board has been rebuilt from steps.
    const elementByStep = new Map<string, string>()
    for (const [elementId, stepId] of Object.entries(elementStepMap)) {
      elementByStep.set(stepId, elementId)
    }

    // Every step any layer has something to say about.
    const stepIds = new Set<string>([
      ...Object.keys(baselineByStep),
      ...Object.keys(stepStates),
      ...Collections.mapBy(dispatches, (d) => d.stepId),
    ])

    for (const stepId of stepIds) {
      const resolved = resolveNodeStatus({
        baseline: baselineByStep[stepId] ?? null,
        runState: stepStates[stepId],
        dispatch: dispatchByStep.get(stepId) ?? null,
      })
      const ring = resolveStatusRing({
        status: resolved.status,
        designStatus: resolved.designStatus,
        palette: theme.palette.statusPalette,
        animated,
      })
      if (ring === null) continue

      // Both regimes, always. See the note above on why neither id is trusted alone.
      rings.set(stepId, ring)
      const elementId = elementByStep.get(stepId)
      if (elementId !== undefined) rings.set(elementId, ring)
    }

    return rings
  }, [elementStepMap, baselineByStep, dispatches, stepStates, theme.palette.statusPalette, animated])
}

export { useBoardStatusRings }
