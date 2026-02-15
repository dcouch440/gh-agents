import { api } from '@/api'
import { store, getActiveId } from './_store'

const fetchAllNotes = async (workflowId?: string): Promise<void> => {
  const wid = workflowId ?? getActiveId()
  if (!wid) return
  try {
    const notes = await api.workflows.getAllNotes(wid)
    const lookup: Record<string, string> = {}
    for (const entry of notes) {
      lookup[entry.step_id] = entry.content
    }
    store.setState({ notesByStep: lookup })
  } catch (e) {
    console.error('[workflowStore] Failed to fetch notes:', e)
  }
}

export { fetchAllNotes }
