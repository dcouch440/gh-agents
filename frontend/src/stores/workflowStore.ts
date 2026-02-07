// ============================================================================
// workflowStore — Hand-written store for workflows + steps + edges + docs
// ============================================================================

import { createStore, createNormalizedMap, nmFromArray, nmSet, nmDelete, toArray, nmGet } from './lib'
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
} from '@/types/workflow'
import type { Document } from '@/types/document'

// ── State ────────────────────────────────────────────────────────────────────

type WorkflowState = {
  items: NormalizedMap<Workflow>
  activeWorkflowId: string | null
  steps: WorkflowStep[]
  edges: WorkflowStepEdge[]
  documentsByStep: Record<string, Document[]>
  loading: boolean
  error: string | null
  dirty: boolean
  lastFetched: number | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const STALE_THRESHOLD_MS = 60_000

const store = createStore<WorkflowState>(() => ({
  items: createNormalizedMap<Workflow>(),
  activeWorkflowId: null,
  steps: [],
  edges: [],
  documentsByStep: {},
  loading: false,
  error: null,
  dirty: false,
  lastFetched: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string =>
  e instanceof Error ? e.message : 'workflows: unknown error'

const getActiveId = (): string | null => store.getState().activeWorkflowId

const EMPTY_DOCS: Document[] = []

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: WorkflowState): Workflow[] => toArray(s.items)

const selectById = (id: string) => (s: WorkflowState): Workflow | undefined =>
  nmGet(s.items, id)

const selectActiveWorkflowId = (s: WorkflowState): string | null => s.activeWorkflowId

const selectSteps = (s: WorkflowState): WorkflowStep[] => s.steps

const selectEdges = (s: WorkflowState): WorkflowStepEdge[] => s.edges

const selectStepDocuments = (stepId: string) => (s: WorkflowState): Document[] =>
  s.documentsByStep[stepId] ?? EMPTY_DOCS

const selectLoading = (s: WorkflowState): boolean => s.loading

const selectError = (s: WorkflowState): string | null => s.error

const selectDirty = (s: WorkflowState): boolean => s.dirty

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
      steps,
      edges,
      documentsByStep: {},
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
    steps: [],
    edges: [],
    documentsByStep: {},
    dirty: false,
  })
}

// ── Steps ────────────────────────────────────────────────────────────────────

const createStep = async (body: CreateStepRequest): Promise<WorkflowStep | null> => {
  const wid = getActiveId()
  if (!wid) return null
  const step = await api.workflows.createStep(wid, body)
  store.setState((s) => ({ steps: [...s.steps, step], dirty: true }))
  return step
}

const updateStep = async (stepId: string, body: UpdateStepRequest): Promise<WorkflowStep | null> => {
  const wid = getActiveId()
  if (!wid) return null
  const step = await api.workflows.updateStep(wid, stepId, body)
  store.setState((s) => ({
    steps: s.steps.map((st) => (st.id === stepId ? step : st)),
    dirty: true,
  }))
  return step
}

const deleteStep = async (stepId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  await api.workflows.deleteStep(wid, stepId)
  store.setState((s) => ({
    steps: s.steps.filter((st) => st.id !== stepId),
    edges: s.edges.filter((e) => e.from_step_id !== stepId && e.to_step_id !== stepId),
    dirty: true,
  }))
}

// ── Edges ────────────────────────────────────────────────────────────────────

const addEdge = async (body: EdgeRequest): Promise<WorkflowStepEdge | null> => {
  const wid = getActiveId()
  if (!wid) return null
  const edge = await api.workflows.createEdge(wid, body)
  store.setState((s) => ({ edges: [...s.edges, edge], dirty: true }))
  return edge
}

const removeEdge = async (edgeId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  await api.workflows.deleteEdge(wid, edgeId)
  store.setState((s) => ({
    edges: s.edges.filter((e) => e.id !== edgeId),
    dirty: true,
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
  selectStepDocuments,
  selectLoading,
  selectError,
  selectDirty,
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
  updateStep,
  deleteStep,
  addEdge,
  removeEdge,
  fetchStepDocuments,
  addStepDocument,
  removeStepDocument,
  setDirty,
  upsert,
}

export type { WorkflowState }
