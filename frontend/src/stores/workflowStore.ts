// ============================================================================
// workflowStore — Hand-written store for workflows + steps + edges + docs
// ============================================================================

import { createStore, createNormalizedMap, nmFromArray, nmSet, nmDelete, toArray, nmGet, logger } from './lib'
import type { NormalizedMap } from './lib'
import { api } from '@/api'
import type {
  Workflow,
  CreateWorkflowRequest,
  UpdateWorkflowRequest,
  WorkflowStep,
  CreateStepRequest,
  UpdateStepRequest,
  WorkflowStepEdge,
  EdgeRequest,
  DocumentDef,
  CreateDocumentDefRequest,
} from '@/types/workflow'
import type { Document } from '@/types/document'

// ── State ────────────────────────────────────────────────────────────────────

type WorkflowState = {
  items: NormalizedMap<Workflow>
  activeWorkflowId: string | null
  steps: NormalizedMap<WorkflowStep>
  edges: NormalizedMap<WorkflowStepEdge>
  documentsByStep: Record<string, Document[]>
  documentDefsByStep: Record<string, DocumentDef[]>
  dirtyStepIds: Set<string>
  loading: boolean
  error: string | null
  dirty: boolean
  lastFetched: number | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const STALE_THRESHOLD_MS = 60_000

const store = logger('workflowStore', createStore<WorkflowState>(() => ({
  items: createNormalizedMap<Workflow>(),
  activeWorkflowId: null,
  steps: createNormalizedMap<WorkflowStep>(),
  edges: createNormalizedMap<WorkflowStepEdge>(),
  documentsByStep: {},
  documentDefsByStep: {},
  dirtyStepIds: new Set<string>(),
  loading: false,
  error: null,
  dirty: false,
  lastFetched: null,
})))

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string =>
  e instanceof Error ? e.message : 'workflows: unknown error'

const getActiveId = (): string | null => store.getState().activeWorkflowId

const EMPTY_DOCS: Document[] = []
const EMPTY_DEFS: DocumentDef[] = []

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: WorkflowState): Workflow[] => toArray(s.items)

const selectById = (id: string) => (s: WorkflowState): Workflow | undefined =>
  nmGet(s.items, id)

const selectActiveWorkflowId = (s: WorkflowState): string | null => s.activeWorkflowId

const selectSteps = (s: WorkflowState): WorkflowStep[] => toArray(s.steps)

const selectEdges = (s: WorkflowState): WorkflowStepEdge[] => toArray(s.edges)

const selectStepById = (id: string | null) => (s: WorkflowState): WorkflowStep | null =>
  id !== null ? nmGet(s.steps, id) ?? null : null

const selectEdgeById = (id: string | null) => (s: WorkflowState): WorkflowStepEdge | null =>
  id !== null ? nmGet(s.edges, id) ?? null : null

const selectStepDocuments = (stepId: string) => (s: WorkflowState): Document[] =>
  s.documentsByStep[stepId] ?? EMPTY_DOCS

const selectStepDocumentDefs = (stepId: string) => (s: WorkflowState): DocumentDef[] =>
  s.documentDefsByStep[stepId] ?? EMPTY_DEFS

const selectLoading = (s: WorkflowState): boolean => s.loading

const selectError = (s: WorkflowState): string | null => s.error

const selectDirty = (s: WorkflowState): boolean => s.dirty

const selectDirtyStepIds = (s: WorkflowState): Set<string> => s.dirtyStepIds

const selectIsStale = (s: WorkflowState): boolean =>
  s.lastFetched === null || Date.now() - s.lastFetched > STALE_THRESHOLD_MS

// ── Workflow CRUD ────────────────────────────────────────────────────────────

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.workflows.list()
    store.setState({ items: nmFromArray(data), loading: false, lastFetched: Date.now() })
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

const fetchIfStale = async (): Promise<void> => {
  if (selectIsStale(store.getState())) {
    await fetchAll()
  }
}

const fetchOne = async (id: string): Promise<Workflow> => {
  const workflow = await api.workflows.get(id)
  store.setState((s) => ({ items: nmSet(s.items, workflow.id, workflow) }))
  return workflow
}

const create = async (body: CreateWorkflowRequest): Promise<Workflow> => {
  const workflow = await api.workflows.create(body)
  store.setState((s) => ({ items: nmSet(s.items, workflow.id, workflow) }))
  return workflow
}

const update = async (id: string, body: UpdateWorkflowRequest): Promise<Workflow> => {
  const workflow = await api.workflows.update(id, body)
  store.setState((s) => ({ items: nmSet(s.items, workflow.id, workflow) }))
  return workflow
}

const remove = async (id: string): Promise<void> => {
  const prev = store.getState()
  store.setState((s) => ({ items: nmDelete(s.items, id) }))
  try {
    await api.workflows.delete(id)
  } catch (e) {
    store.setState({ items: prev.items, error: extractError(e) })
    throw e
  }
}

// ── Active Workflow Context ──────────────────────────────────────────────────

const loadWorkflow = async (id: string): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const [workflow, steps, edges] = await Promise.all([
      api.workflows.get(id),
      api.workflows.listSteps(id),
      api.workflows.listEdges(id),
    ])
    store.setState((s) => ({
      items: nmSet(s.items, workflow.id, workflow),
      activeWorkflowId: id,
      steps: nmFromArray(steps),
      edges: nmFromArray(edges),
      documentsByStep: {},
      documentDefsByStep: {},
      dirtyStepIds: new Set<string>(),
      loading: false,
      dirty: false,
    }))
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

const clearActive = (): void => {
  store.setState({
    activeWorkflowId: null,
    steps: createNormalizedMap<WorkflowStep>(),
    edges: createNormalizedMap<WorkflowStepEdge>(),
    documentsByStep: {},
    documentDefsByStep: {},
    dirtyStepIds: new Set<string>(),
    dirty: false,
  })
}

// ── Steps ────────────────────────────────────────────────────────────────────

const createStep = async (body: CreateStepRequest): Promise<WorkflowStep | null> => {
  const wid = getActiveId()
  if (!wid) return null
  const step = await api.workflows.createStep(wid, body)
  store.setState((s) => ({ steps: nmSet(s.steps, step.id, step) }))
  return step
}

const patchStepLocal = (stepId: string, partial: Partial<WorkflowStep>): void => {
  store.setState((s) => {
    const existing = nmGet(s.steps, stepId)
    if (!existing) return {}

    // Skip if all values in partial are already identical
    const keys = Object.keys(partial) as (keyof WorkflowStep)[]
    const hasChange = keys.some((k) => !Object.is(existing[k], partial[k]))
    if (!hasChange) return {}

    const nextDirty = new Set(s.dirtyStepIds)
    nextDirty.add(stepId)
    return {
      steps: nmSet(s.steps, stepId, { ...existing, ...partial }),
      dirtyStepIds: nextDirty,
      dirty: true,
    }
  })
}

/** Update step data locally without marking it dirty (for auto-derived values). */
const patchStepSilent = (stepId: string, partial: Partial<WorkflowStep>): void => {
  store.setState((s) => {
    const existing = nmGet(s.steps, stepId)
    if (!existing) return {}

    const keys = Object.keys(partial) as (keyof WorkflowStep)[]
    const hasChange = keys.some((k) => !Object.is(existing[k], partial[k]))
    if (!hasChange) return {}

    return {
      steps: nmSet(s.steps, stepId, { ...existing, ...partial }),
    }
  })
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

// ── Save / Revert ───────────────────────────────────────────────────────────

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

// ── Edges ────────────────────────────────────────────────────────────────────

const addEdge = async (body: EdgeRequest): Promise<WorkflowStepEdge | null> => {
  const wid = getActiveId()
  if (!wid) return null
  const edge = await api.workflows.createEdge(wid, body)
  store.setState((s) => ({ edges: nmSet(s.edges, edge.id, edge) }))
  return edge
}

const removeEdge = async (edgeId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  await api.workflows.deleteEdge(wid, edgeId)
  store.setState((s) => ({
    edges: nmDelete(s.edges, edgeId),
  }))
}

// ── Step Documents ───────────────────────────────────────────────────────────

const fetchStepDocuments = async (stepId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  try {
    const docs = await api.workflows.listStepDocuments(wid, stepId)
    store.setState((s) => ({
      documentsByStep: { ...s.documentsByStep, [stepId]: docs },
    }))
  } catch (e) {
    store.setState({ error: extractError(e) })
  }
}

const addStepDocument = async (stepId: string, docId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  await api.workflows.addStepDocument(wid, stepId, docId)
  await fetchStepDocuments(stepId)
}

const removeStepDocument = async (stepId: string, docId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  await api.workflows.removeStepDocument(wid, stepId, docId)
  await fetchStepDocuments(stepId)
}

// ── Document Defs ───────────────────────────────────────────────────────────

const fetchDocumentDefs = async (stepId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  try {
    const defs = await api.workflows.listDocumentDefs(wid, stepId)
    store.setState((s) => ({
      documentDefsByStep: { ...s.documentDefsByStep, [stepId]: defs },
    }))
  } catch (e) {
    store.setState({ error: extractError(e) })
  }
}

const createDocumentDef = async (stepId: string, body: CreateDocumentDefRequest): Promise<DocumentDef | null> => {
  const wid = getActiveId()
  if (!wid) return null
  const def = await api.workflows.createDocumentDef(wid, stepId, body)
  await fetchDocumentDefs(stepId)
  return def
}

const deleteDocumentDef = async (stepId: string, defId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  await api.workflows.deleteDocumentDef(wid, stepId, defId)
  await fetchDocumentDefs(stepId)
}

// ── Sync / Utility ───────────────────────────────────────────────────────────

const setDirty = (dirty: boolean): void => {
  store.setState({ dirty })
}

const upsert = (workflow: Workflow): void => {
  store.setState((s) => ({ items: nmSet(s.items, workflow.id, workflow) }))
}

// ── Export ────────────────────────────────────────────────────────────────────

export const workflowStore = {
  store,
  selectAll,
  selectById,
  selectActiveWorkflowId,
  selectSteps,
  selectEdges,
  selectStepById,
  selectEdgeById,
  selectStepDocuments,
  selectStepDocumentDefs,
  selectLoading,
  selectError,
  selectDirty,
  selectDirtyStepIds,
  selectIsStale,
  fetchAll,
  fetchIfStale,
  fetchOne,
  create,
  update,
  remove,
  loadWorkflow,
  clearActive,
  createStep,
  patchStepLocal,
  patchStepSilent,
  updateStep,
  saveAllDirtySteps,
  revertSteps,
  deleteStep,
  addEdge,
  removeEdge,
  fetchStepDocuments,
  addStepDocument,
  removeStepDocument,
  fetchDocumentDefs,
  createDocumentDef,
  deleteDocumentDef,
  setDirty,
  upsert,
}

export type { WorkflowState }
