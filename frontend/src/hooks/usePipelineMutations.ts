import { useState, useCallback } from 'react'
import { api } from '../api'
import type { Task, ApproveGateRequest, CreateSideTaskRequest } from '../types'

type RenderStageResponse = {
  rendered_prompt: string
}

const useApproveGate = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (runId: string, body?: ApproveGateRequest): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.post(`/pipeline-runs/${runId}/approve`, body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to approve gate'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useRenderStage = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (pipelineId: string, stageNumber: number): Promise<RenderStageResponse> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<RenderStageResponse>(`/pipelines/${pipelineId}/stages/${stageNumber}/render`)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to render stage'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useSideTasks = () => {
  const [tasks, setTasks] = useState<Task[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async (pipelineId: string, stageNumber: number): Promise<Task[]> => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.get<Task[]>(`/pipelines/${pipelineId}/stages/${stageNumber}/side-tasks`)
      setTasks(data)
      return data
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to load side tasks'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  const create = useCallback(async (pipelineId: string, stageNumber: number, body: CreateSideTaskRequest): Promise<Task> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<Task>(`/pipelines/${pipelineId}/stages/${stageNumber}/side-tasks`, body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create side task'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  const remove = useCallback(async (pipelineId: string, stageNumber: number, sideTaskId: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(`/pipelines/${pipelineId}/stages/${stageNumber}/side-tasks/${sideTaskId}`)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete side task'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { tasks, load, create, remove, loading, error }
}

export { useApproveGate, useRenderStage, useSideTasks }
