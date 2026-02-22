import { api } from '@/api'
import { store, getActiveId } from './_store'
import type { StepQuestionState } from '@/types/workflow'

const fetchQuestionStates = async (workflowId?: string): Promise<void> => {
  const wid = workflowId ?? getActiveId()
  if (!wid) return
  try {
    const states = await api.workflows.listQuestionStates(wid)
    const lookup: Record<string, StepQuestionState> = {}
    for (const entry of states) {
      lookup[entry.step_id] = entry
    }
    store.setState({ questionStateByStep: lookup })
  } catch (e) {
    console.error('[workflowStore] Failed to fetch question states:', e)
  }
}

export { fetchQuestionStates }
