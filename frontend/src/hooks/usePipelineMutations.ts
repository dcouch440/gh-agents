import { useState, useCallback } from 'react'
import { api } from '@/api'
import { API } from '@/constants'
import type { Task, ApproveGateRequest, CreateSideTaskRequest } from '@/types'

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
      await api.post(API.PIPELINE_RUN_APPROVE(runId), body)
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
      return await api.post<RenderStageResponse>(API.PIPELINE_STAGE_RENDER(pipelineId, stageNumber))
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
      const data = await api.get<Task[]>(API.PIPELINE_SIDE_TASKS(pipelineId, stageNumber))
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
      return await api.post<Task>(API.PIPELINE_SIDE_TASKS(pipelineId, stageNumber), body)
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
      await api.del(API.PIPELINE_SIDE_TASK(pipelineId, stageNumber, sideTaskId))
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
