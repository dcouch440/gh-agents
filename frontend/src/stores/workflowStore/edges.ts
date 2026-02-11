import { nmSet, nmDelete } from '../lib'
import { api } from '@/api'
import type { WorkflowStepEdge, EdgeRequest } from '@/types/workflow'
import { store, getActiveId } from './_store'

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

export { addEdge, removeEdge }
