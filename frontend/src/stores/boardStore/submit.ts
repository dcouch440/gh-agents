import { batch, nmSet, nmDelete, extractError } from '../lib'
import type { NormalizedMap } from '../lib'
import { api } from '@/api'
import { store } from './_store'
import { workflowStore } from '../workflowStore'
import { Collections } from '@/utils/collections'
import type { PhaseZeroResponse, PhaseZeroStep } from '@/types/board'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

// ── Element Map Builders ───────────────────────────────────────────────────

/**
 * Merge newly created/updated step mappings into the existing element-to-step map.
 * Uses `Collections.toLookupMap` for a single-pass build, then spreads over the
 * existing map so prior mappings are preserved. O(n) where n = new entries.
 */
const mergeElementStepMap = (
  existing: Readonly<Record<string, string>>,
  phaseZero: PhaseZeroResponse,
): Readonly<Record<string, string>> => {
  const allSteps: readonly PhaseZeroStep[] = [...phaseZero.created_steps, ...phaseZero.updated_steps]
  if (allSteps.length === 0 && phaseZero.deleted_steps.length === 0) return existing

  const newEntries = Collections.toLookupMap(
    allSteps,
    (s: PhaseZeroStep) => s.element_id,
    (s: PhaseZeroStep) => s.id,
  )

  const result: Record<string, string> = { ...existing }
  for (const [key, value] of newEntries) {
    result[key] = value
  }

  // Remove deleted step mappings
  const deleted = phaseZero.deleted_steps
  for (let i = 0, n = deleted.length; i < n; i++) {
    delete result[deleted[i]!]
  }

  return result
}

/**
 * Merge newly created edge mappings into the existing element-to-edge map.
 * O(n) where n = new entries.
 */
const mergeElementEdgeMap = (
  existing: Readonly<Record<string, string>>,
  phaseZero: PhaseZeroResponse,
): Readonly<Record<string, string>> => {
  const created = phaseZero.created_edges
  if (created.length === 0 && phaseZero.deleted_edges.length === 0) return existing

  const result: Record<string, string> = { ...existing }
  for (let i = 0, n = created.length; i < n; i++) {
    const pair = created[i]!
    result[pair.element_id] = pair.edge_id
  }

  // Remove deleted edge mappings
  const deleted = phaseZero.deleted_edges
  for (let i = 0, n = deleted.length; i < n; i++) {
    delete result[deleted[i]!]
  }

  return result
}

// ── Selective Sync ─────────────────────────────────────────────────────────

/**
 * Apply Phase 0 results into workflowStore via selective `nmSet`/`nmDelete`.
 *
 * Only touches entries that actually changed — existing step/edge object
 * references in the NormalizedMap are preserved, so unchanged canvas nodes
 * do not re-render. All mutations are batched into a single notification.
 */
const syncPhaseZero = (phaseZero: PhaseZeroResponse): void => {
  batch(() => {
    const wfStore = workflowStore.store

    wfStore.setState((s) => {
      let nextSteps: NormalizedMap<WorkflowStep> = s.steps
      let nextEdges: NormalizedMap<WorkflowStepEdge> = s.edges

      // Upsert created steps
      const created = phaseZero.created_steps
      for (let i = 0, n = created.length; i < n; i++) {
        const step = created[i]!
        nextSteps = nmSet(nextSteps, step.id, step as WorkflowStep)
      }

      // Upsert updated steps
      const updated = phaseZero.updated_steps
      for (let i = 0, n = updated.length; i < n; i++) {
        const step = updated[i]!
        nextSteps = nmSet(nextSteps, step.id, step as WorkflowStep)
      }

      // Delete removed steps
      const deletedSteps = phaseZero.deleted_steps
      for (let i = 0, n = deletedSteps.length; i < n; i++) {
        nextSteps = nmDelete(nextSteps, deletedSteps[i]!)
      }

      // Delete removed edges
      const deletedEdges = phaseZero.deleted_edges
      for (let i = 0, n = deletedEdges.length; i < n; i++) {
        nextEdges = nmDelete(nextEdges, deletedEdges[i]!)
      }

      // Only return new references if something changed
      const stepsChanged = nextSteps !== s.steps
      const edgesChanged = nextEdges !== s.edges
      if (!stepsChanged && !edgesChanged) return {}
      return {
        ...(stepsChanged ? { steps: nextSteps } : {}),
        ...(edgesChanged ? { edges: nextEdges } : {}),
      }
    })
  })
}

// ── Submit Action ──────────────────────────────────────────────────────────

/**
 * Submit the current Excalidraw board to the backend and apply Phase 0 results.
 *
 * 1. POST raw elements to `/workflows/:id/board/submit`
 * 2. Selectively sync created/updated/deleted steps and edges into workflowStore
 * 3. Accumulate element-to-step/edge mappings for future lookups
 *
 * @param workflowId - The active workflow UUID.
 * @param elements - Raw Excalidraw elements array (read from Excalidraw ref).
 */
const submitBoard = async (workflowId: string, elements: ReadonlyArray<unknown>): Promise<void> => {
  store.setState({ status: 'submitting', error: null })

  try {
    const response = await api.workflows.submitBoard(workflowId, elements)

    syncPhaseZero(response.phase_zero)

    const prev = store.getState()
    store.setState({
      status: 'success',
      error: null,
      lastResponse: response,
      isFirstSubmit: response.is_first_submit,
      elementStepMap: mergeElementStepMap(prev.elementStepMap, response.phase_zero),
      elementEdgeMap: mergeElementEdgeMap(prev.elementEdgeMap, response.phase_zero),
    })
  } catch (err) {
    store.setState({
      status: 'error',
      error: extractError('Board submit failed', err),
    })
  }
}

/** Reset the board store to its initial state (e.g. when switching workflows). */
const resetBoard = (): void => {
  store.setState({
    status: 'idle',
    error: null,
    lastResponse: null,
    isFirstSubmit: false,
    elementStepMap: {},
    elementEdgeMap: {},
  })
}

export { submitBoard, resetBoard, syncPhaseZero, mergeElementStepMap, mergeElementEdgeMap }
