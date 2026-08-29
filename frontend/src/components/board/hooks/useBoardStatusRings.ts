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
 * Driven from the steps that actually have status, not from
 * `boardStore.elementStepMap`. That map is populated only from a submit
 * response and rehydrated from `canvas_snapshots.last_response_json`, which is
 * never written — so it is empty on every page load and cannot be the spine of
 * this lookup. It is still consulted as an override when present.
 *
 * The fallback is the real invariant: the server builds board elements *from*
 * steps (`build_canvas_elements`), so a box's element id is its step id. A box
 * the user has drawn but not yet submitted matches no step, resolves to idle,
 * and correctly gets no ring.
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

    // step id -> element id, for the rare case the two ever diverge.
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
      if (ring !== null) rings.set(elementByStep.get(stepId) ?? stepId, ring)
    }

    return rings
  }, [elementStepMap, baselineByStep, dispatches, stepStates, theme.palette.statusPalette, animated])
}

export { useBoardStatusRings }
