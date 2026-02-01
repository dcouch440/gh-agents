import { useState, useEffect, useCallback } from 'react'
import type { Agent } from '../types/agent'
import { USE_MOCK_DATA } from '../constants'
import { mock } from '../mock'
import { api } from '../api'

type AgentsResponse = {
  stats: { orchestrators: number; workers: number; utilities: number }
  agents: Agent[]
}

const useAgents = () => {
  const [agents, setAgents] = useState<Agent[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = USE_MOCK_DATA
        ? await mock.getAgents()
        : (await api.get<AgentsResponse>('/agents')).agents
      setAgents(data)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load agents')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      await load()
      if (cancelled) return
    }
    run()
    return () => { cancelled = true }
  }, [load])

  return { agents, loading, error, reload: load }
}

const useAgent = (id: string | null) => {
  const [agent, setAgent] = useState<Agent | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!id) {
      setAgent(null)
      setLoading(false)
      return
    }

    let cancelled = false
    const load = async () => {
      setLoading(true)
      setError(null)
      try {
        const data = USE_MOCK_DATA
          ? await mock.getAgent(id)
          : await api.get<Agent>(`/agents/${id}`)
        if (!cancelled) setAgent(data)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load agent')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    load()
    return () => { cancelled = true }
  }, [id])

  return { agent, loading, error }
}

export { useAgents, useAgent }
