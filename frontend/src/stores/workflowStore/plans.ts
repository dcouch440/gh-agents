import { api } from '@/api'
import { store, getActiveId } from './_store'

const fetchAllPlans = async (workflowId?: string): Promise<void> => {
  const wid = workflowId ?? getActiveId()
  if (!wid) return
  try {
    const plans = await api.workflows.getAllPlans(wid)
    const lookup: Record<string, string> = {}
    for (const entry of plans) {
      lookup[entry.step_id] = entry.content
    }
    store.setState({ planByStep: lookup })
  } catch (e) {
    console.error('[workflowStore] Failed to fetch plans:', e)
  }
}

export { fetchAllPlans }
