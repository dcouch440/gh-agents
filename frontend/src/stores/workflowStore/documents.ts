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

// ── Document Content (generated content from workflow runs) ──────────

const fetchDocumentContent = async (stepId: string): Promise<void> => {
  const defs = store.getState().documentDefsByStep[stepId] ?? []
  const withDocId = defs.filter((d): d is typeof d & { document_id: string } => d.document_id !== null)
  if (withDocId.length === 0) return

  try {
    const results = await Promise.all(
      withDocId.map(async (def) => {
        const doc = await api.documents.get(def.document_id)
        return { defId: def.id, content: doc.content }
      }),
    )
    store.setState((s) => {
      const updated = { ...s.documentContentByDefId }
      for (const { defId, content } of results) {
        updated[defId] = content
      }
      return { documentContentByDefId: updated }
    })
  } catch {
    // Non-fatal: documents display as empty until next run
  }
}

export { fetchStepDocuments, addStepDocument, removeStepDocument, fetchDocumentDefs, createDocumentDef, deleteDocumentDef, fetchDocumentContent }
