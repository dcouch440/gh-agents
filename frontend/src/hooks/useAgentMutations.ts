import { useState, useCallback } from 'react'
import { api } from '@/api'
import { API } from '@/constants'
import type { Agent, CreateAgentRequest, UpdateAgentRequest, AgentToolsResponse, AgentContextResponse } from '@/types'
import type { Tool } from '@/types'
import type { DocumentListItem } from '@/types'

const useCreateAgent = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: CreateAgentRequest): Promise<Agent> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<Agent>(API.AGENTS, body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create agent'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useUpdateAgent = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string, body: UpdateAgentRequest): Promise<Agent> => {
    setLoading(true)
    setError(null)
    try {
      return await api.patch<Agent>(API.AGENT(id), body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to update agent'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useDeleteAgent = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.AGENT(id))
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete agent'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useAgentTools = () => {
  const [tools, setTools] = useState<Tool[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async (agentId: string): Promise<Tool[]> => {
    setLoading(true)
    setError(null)
    try {
      const res = await api.get<AgentToolsResponse>(API.AGENT_TOOLS(agentId))
      setTools(res.tools)
      return res.tools
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to load agent tools'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  const save = useCallback(async (agentId: string, toolIds: string[]): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.put(API.AGENT_TOOLS(agentId), toolIds)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to save agent tools'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { tools, load, save, loading, error }
}

const useAgentContextDocs = () => {
  const [docs, setDocs] = useState<DocumentListItem[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async (agentId: string): Promise<DocumentListItem[]> => {
    setLoading(true)
    setError(null)
    try {
      const res = await api.get<AgentContextResponse>(API.AGENT_CONTEXT(agentId))
      setDocs(res.documents)
      return res.documents
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to load agent context'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  const save = useCallback(async (agentId: string, docIds: string[]): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.put(API.AGENT_CONTEXT(agentId), docIds)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to save agent context'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { docs, load, save, loading, error }
}

export { useCreateAgent, useUpdateAgent, useDeleteAgent, useAgentTools, useAgentContextDocs }
