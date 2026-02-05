import { useState, useCallback } from 'react'
import { api } from '@/api'
import type {
  ToolRouter,
  CreateToolRouterRequest,
  UpdateToolRouterRequest,
  SetRouterToolsRequest,
  Tool,
} from '@/types'

const useToolRouterMutations = () => {
  const [creating, setCreating] = useState(false)
  const [updating, setUpdating] = useState(false)
  const [deleting, setDeleting] = useState(false)
  const [loadingTools, setLoadingTools] = useState(false)
  const [savingTools, setSavingTools] = useState(false)
  const [toolsError, setToolsError] = useState<string | null>(null)

  const createRouter = useCallback(
    async (body: CreateToolRouterRequest): Promise<ToolRouter> => {
      setCreating(true)
      try {
        return await api.toolRouters.create(body)
      } finally {
        setCreating(false)
      }
    },
    [],
  )

  const updateRouter = useCallback(
    async (id: string, body: UpdateToolRouterRequest): Promise<ToolRouter> => {
      setUpdating(true)
      try {
        return await api.toolRouters.update(id, body)
      } finally {
        setUpdating(false)
      }
    },
    [],
  )

  const deleteRouter = useCallback(async (id: string): Promise<void> => {
    setDeleting(true)
    try {
      await api.toolRouters.delete(id)
    } finally {
      setDeleting(false)
    }
  }, [])

  const loadRouterTools = useCallback(async (id: string): Promise<Tool[]> => {
    setLoadingTools(true)
    setToolsError(null)
    try {
      return await api.toolRouters.getTools(id)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to load router tools'
      setToolsError(msg)
      throw e
    } finally {
      setLoadingTools(false)
    }
  }, [])

  const saveRouterTools = useCallback(
    async (id: string, body: SetRouterToolsRequest): Promise<void> => {
      setSavingTools(true)
      setToolsError(null)
      try {
        await api.toolRouters.setTools(id, body)
      } catch (e) {
        const msg = e instanceof Error ? e.message : 'Failed to save router tools'
        setToolsError(msg)
        throw e
      } finally {
        setSavingTools(false)
      }
    },
    [],
  )

  return {
    createRouter,
    creating,
    updateRouter,
    updating,
    deleteRouter,
    deleting,
    loadRouterTools,
    loadingTools,
    saveRouterTools,
    savingTools,
    toolsError,
  }
}

export { useToolRouterMutations }
