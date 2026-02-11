import { api } from '@/api'
import type { DocumentDef, CreateDocumentDefRequest } from '@/types/workflow'
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

// ── Document Defs ───────────────────────────────────────────────────

const fetchDocumentDefs = async (stepId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  try {
    const defs = await api.workflows.listDocumentDefs(wid, stepId)
    store.setState((s) => ({
      documentDefsByStep: { ...s.documentDefsByStep, [stepId]: defs },
    }))
  } catch (e) {
    store.setState({ error: extractError('workflows', e) })
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

export { fetchStepDocuments, addStepDocument, removeStepDocument, fetchDocumentDefs, createDocumentDef, deleteDocumentDef }
