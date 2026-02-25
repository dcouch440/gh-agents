import { nmSet, nmGet, nmDelete } from '../lib'
import type { NormalizedMap } from '../lib'
import { api } from '@/api'
import type { WorkflowStep, CreateStepRequest, UpdateStepRequest } from '@/types/workflow'
import { store, getActiveId } from './_store'
import type { WorkflowState } from './types'
import { loadWorkflow } from './workflows'

/** Merge a partial update into a step if any values actually changed. Returns null if no change. */
const _mergeStepPartial = (
  s: WorkflowState,
  stepId: string,
  partial: Partial<WorkflowStep>,
): { steps: NormalizedMap<WorkflowStep> } | null => {
  const existing = nmGet(s.steps, stepId)
  if (!existing) return null
  const keys = Object.keys(partial) as (keyof WorkflowStep)[]
  const hasChange = keys.some((k) => !Object.is(existing[k], partial[k]))
  if (!hasChange) return null
  return { steps: nmSet(s.steps, stepId, { ...existing, ...partial }) }
}

const createStep = async (body: CreateStepRequest): Promise<WorkflowStep | null> => {
  const wid = getActiveId()
  if (!wid) return null
  const step = await api.workflows.createStep(wid, body)
  store.setState((s) => ({ steps: nmSet(s.steps, step.id, step) }))
  return step
}

const patchStepLocal = (stepId: string, partial: Partial<WorkflowStep>): void => {
  store.setState((s) => {
    const merged = _mergeStepPartial(s, stepId, partial)
    if (!merged) return {}
    const nextDirty = new Set(s.dirtyStepIds)
    nextDirty.add(stepId)
    return { ...merged, dirtyStepIds: nextDirty, dirty: true }
  })
}

/** Update step data locally without marking it dirty (for auto-derived values). */
const patchStepSilent = (stepId: string, partial: Partial<WorkflowStep>): void => {
  store.setState((s) => _mergeStepPartial(s, stepId, partial) ?? {})
}

const updateStep = async (stepId: string, body: UpdateStepRequest): Promise<WorkflowStep | null> => {
  const wid = getActiveId()
  if (!wid) return null
  const step = await api.workflows.updateStep(wid, stepId, body)
  store.setState((s) => {
    if (s.dirtyStepIds.has(stepId)) {
      const local = nmGet(s.steps, stepId)
      if (local) {
        return { steps: nmSet(s.steps, stepId, { ...step, ...local }) }
      }
    }
    return { steps: nmSet(s.steps, stepId, step) }
  })
  return step
}

const deleteStep = async (stepId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  await api.workflows.deleteStep(wid, stepId)
  store.setState((s) => {
    let nextEdges = s.edges
    for (const [edgeId, edge] of s.edges.byId) {
      if (edge.from_step_id === stepId || edge.to_step_id === stepId) {
        nextEdges = nmDelete(nextEdges, edgeId)
      }
    }
    const nextDirty = new Set(s.dirtyStepIds)
    nextDirty.delete(stepId)
    return {
      steps: nmDelete(s.steps, stepId),
      edges: nextEdges,
      dirtyStepIds: nextDirty,
      dirty: nextDirty.size > 0,
    }
  })
}

/** Remove a step from local state only (no API call). Used when canvas elements are deleted before submit. */
const removeStepLocal = (stepId: string): void => {
  store.setState((s) => {
    let nextEdges = s.edges
    for (const [edgeId, edge] of s.edges.byId) {
      if (edge.from_step_id === stepId || edge.to_step_id === stepId) {
        nextEdges = nmDelete(nextEdges, edgeId)
      }
    }
    const nextDirty = new Set(s.dirtyStepIds)
    nextDirty.delete(stepId)
    return {
      steps: nmDelete(s.steps, stepId),
      edges: nextEdges,
      dirtyStepIds: nextDirty,
      dirty: nextDirty.size > 0,
    }
  })
}

// ── Save / Revert ───────────────────────────────────────────────────

const saveAllDirtySteps = async (): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  const { dirtyStepIds, steps } = store.getState()
  if (dirtyStepIds.size === 0) return

  const ids = [...dirtyStepIds]
  const promises = ids.map((stepId) => {
    const step = nmGet(steps, stepId)
    if (!step) return Promise.resolve(null)
    const body: UpdateStepRequest = {
      name: step.name ?? undefined,
      agent_id: step.agent_id,
      prompt_template: step.prompt_template,
      prompt_template_id: step.prompt_template_id ?? undefined,
      output_schema_id: step.output_schema_id ?? undefined,
      output_variable_name: step.output_variable_name ?? undefined,
      system_prompt_suffix: step.system_prompt_suffix ?? undefined,
    }
    return api.workflows.updateStep(wid, stepId, body)
  })

  const results = await Promise.all(promises)

  store.setState((s) => {
    let nextSteps = s.steps
    for (const updated of results) {
      if (updated) {
        nextSteps = nmSet(nextSteps, updated.id, updated)
      }
    }
    return {
      steps: nextSteps,
      dirtyStepIds: new Set<string>(),
      dirty: false,
    }
  })
}

const revertSteps = async (): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  await loadWorkflow(wid)
}

export { createStep, patchStepLocal, patchStepSilent, updateStep, deleteStep, removeStepLocal, saveAllDirtySteps, revertSteps }
