import { useState, useEffect, useCallback } from 'react'
import type { Agent, AgentsResponse } from '@/types/agent'
import { API, USE_MOCK_DATA } from '@/constants'
import { mock } from '@/mock'
import { api } from '@/api'

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
        : (await api.get<AgentsResponse>(API.AGENTS)).agents
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
    void run()
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
          : await api.get<Agent>(API.AGENT(id))
        if (!cancelled) setAgent(data)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load agent')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void load()
    return () => { cancelled = true }
  }, [id])

  return { agent, loading, error }
}

export { useAgents, useAgent }
