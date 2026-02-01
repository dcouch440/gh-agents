import { useState, useCallback } from 'react'
import { api } from '../api'
import type { Agent, CreateAgentRequest, UpdateAgentRequest, Tool, Document } from '../types'

const useCreateAgent = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: CreateAgentRequest): Promise<Agent> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<Agent>('/agents', body)
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
      return await api.patch<Agent>(`/agents/${id}`, body)
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
      await api.del(`/agents/${id}`)
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
      const data = await api.get<Tool[]>(`/agents/${agentId}/tools`)
      setTools(data)
      return data
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
      await api.put(`/agents/${agentId}/tools`, toolIds)
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
  const [docs, setDocs] = useState<Document[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async (agentId: string): Promise<Document[]> => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.get<Document[]>(`/agents/${agentId}/context`)
      setDocs(data)
      return data
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
      await api.put(`/agents/${agentId}/context`, docIds)
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
