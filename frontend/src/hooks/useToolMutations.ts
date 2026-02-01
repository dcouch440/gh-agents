import { useState, useCallback } from 'react'
import { api } from '@/api'
import { API } from '@/constants'
import type { Tool, CreateToolRequest, UpdateToolRequest } from '@/types'

const useCreateTool = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: CreateToolRequest): Promise<Tool> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<Tool>(API.TOOLS, body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create tool'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useUpdateTool = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string, body: UpdateToolRequest): Promise<Tool> => {
    setLoading(true)
    setError(null)
    try {
      return await api.patch<Tool>(API.TOOL(id), body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to update tool'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useDeleteTool = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.TOOL(id))
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete tool'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

export { useCreateTool, useUpdateTool, useDeleteTool }
