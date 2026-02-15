import { nmFromArray, nmSet, nmDelete, createNormalizedMap } from '../lib'
import { api } from '@/api'
import type { Workflow, CreateWorkflowRequest, UpdateWorkflowRequest, WorkflowStep, WorkflowStepEdge } from '@/types/workflow'
import { extractError } from '../lib'
import { store, STALE_THRESHOLD_MS } from './_store'

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.workflows.list()
    store.setState({ items: nmFromArray(data), loading: false, lastFetched: Date.now() })
  } catch (e) {
    store.setState({ loading: false, error: extractError('workflows', e) })
  }
}

const fetchIfStale = async (): Promise<void> => {
  const { lastFetched } = store.getState()
  if (lastFetched === null || Date.now() - lastFetched > STALE_THRESHOLD_MS) {
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
    store.setState({ items: prev.items, error: extractError('workflows', e) })
    throw e
  }
}

const loadWorkflow = async (id: string): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const [workflow, steps, edges] = await Promise.all([api.workflows.get(id), api.workflows.listSteps(id), api.workflows.listEdges(id)])
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
    store.setState({ loading: false, error: extractError('workflows', e) })
  }
}

const clearActive = (): void => {
  store.setState({
    activeWorkflowId: null,
    steps: createNormalizedMap<WorkflowStep>(),
    edges: createNormalizedMap<WorkflowStepEdge>(),
    documentsByStep: {},
    documentDefsByStep: {},
    notesByStep: {},
    dirtyStepIds: new Set<string>(),
    dirty: false,
  })
}

const upsert = (workflow: Workflow): void => {
  store.setState((s) => ({ items: nmSet(s.items, workflow.id, workflow) }))
}

const setDirty = (dirty: boolean): void => {
  store.setState({ dirty })
}

export { fetchAll, fetchIfStale, fetchOne, create, update, remove, loadWorkflow, clearActive, upsert, setDirty }
