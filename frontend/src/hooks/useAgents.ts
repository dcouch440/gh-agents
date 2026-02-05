import { useState, useEffect, useCallback } from 'react'
import type { Agent } from '@/types/agent'
import { api } from '@/api'

const useAgents = () => {
  const [agents, setAgents] = useState<Agent[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.agents.list()
      setAgents(data.agents)
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

  const load = useCallback(async () => {
    if (!id) {
      setAgent(null)
      setLoading(false)
      return
    }
    setLoading(true)
    setError(null)
    try {
      const data = await api.agents.get(id)
      setAgent(data)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load agent')
    } finally {
      setLoading(false)
    }
  }, [id])

  useEffect(() => {
    if (!id) {
      setAgent(null)
      setLoading(false)
      return
    }

    let cancelled = false
    const run = async () => {
      setLoading(true)
      setError(null)
      try {
        const data = await api.agents.get(id)
        if (!cancelled) setAgent(data)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load agent')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void run()
    return () => { cancelled = true }
  }, [id])

  return { agent, loading, error, reload: load }
}

export { useAgents, useAgent }
