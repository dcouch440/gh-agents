import { api } from '@/api'
import type { CreateRosterAgentRequest } from '@/types/workflow'
import { extractError } from '../lib'
import { store, getActiveId } from './_store'

const fetchRoster = async (stepId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  try {
    const agents = await api.workflows.listRosterAgents(wid, stepId)
    store.setState((s) => ({
      rosterByStep: { ...s.rosterByStep, [stepId]: agents },
    }))
  } catch (e) {
    store.setState({ error: extractError('workflows', e) })
  }
}

const createRosterAgent = async (stepId: string, body: CreateRosterAgentRequest): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  await api.workflows.createRosterAgent(wid, stepId, body)
  await fetchRoster(stepId)
}

const deleteRosterAgent = async (stepId: string, agentId: string): Promise<void> => {
  const wid = getActiveId()
  if (!wid) return
  await api.workflows.deleteRosterAgent(wid, stepId, agentId)
  await fetchRoster(stepId)
}

export { fetchRoster, createRosterAgent, deleteRosterAgent }
