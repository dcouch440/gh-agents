import { api } from '@/api'
import { extractError } from '../lib'
import { store, getActiveId } from './_store'

const fetchStepDocuments = async (stepId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  try {
    const docs = await api.workflows.listStepDocuments(wid, stepId)
    store.setState((s) => ({
      documentsByStep: { ...s.documentsByStep, [stepId]: docs },
    }))
  } catch (e) {
    store.setState({ error: extractError('workflows', e) })
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

export { fetchStepDocuments, addStepDocument, removeStepDocument }
