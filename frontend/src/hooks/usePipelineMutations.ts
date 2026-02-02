import { useState, useCallback } from 'react'
import { api } from '@/api'
import { API } from '@/constants'
import { usePipelineContext } from '@/hooks/usePipelineContext'
import type { Task, ApproveGateRequest, CreateSideTaskRequest } from '@/types'
import type { Pipeline, PipelineStage, PipelineRun, StageMember, CreateStageMemberRequest } from '@/types/pipeline'

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

const useCreatePipeline = () => {
  const { reload } = usePipelineContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: { name: string; stages: PipelineStage[] }): Promise<Pipeline> => {
    setLoading(true)
    setError(null)
    try {
      const pipeline = await api.post<Pipeline>(API.PIPELINES, body)
      reload()
      return pipeline
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create pipeline'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

const useUpdatePipeline = () => {
  const { reload } = usePipelineContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string, body: Partial<Pipeline>): Promise<Pipeline> => {
    setLoading(true)
    setError(null)
    try {
      const pipeline = await api.put<Pipeline>(API.PIPELINE(id), body)
      reload()
      return pipeline
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to update pipeline'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

const useDeletePipeline = () => {
  const { reload } = usePipelineContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (id: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.PIPELINE(id))
      reload()
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete pipeline'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

const useStartPipelineRun = () => {
  const { reload } = usePipelineContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: { pipeline_id: string; initial_task: string }): Promise<PipelineRun> => {
    setLoading(true)
    setError(null)
    try {
      const run = await api.post<PipelineRun>(API.PIPELINE_RUNS, body)
      reload()
      return run
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to start pipeline run'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

const useCancelPipelineRun = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (runId: string): Promise<{ status: string }> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<{ status: string }>(API.PIPELINE_RUN_CANCEL(runId))
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to cancel pipeline run'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

const useAddStageMember = () => {
  const { reload } = usePipelineContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (pipelineId: string, stageNumber: number, body: CreateStageMemberRequest): Promise<StageMember> => {
    setLoading(true)
    setError(null)
    try {
      const member = await api.post<StageMember>(API.STAGE_MEMBERS(pipelineId, stageNumber), body)
      reload()
      return member
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to add stage member'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

const useRemoveStageMember = () => {
  const { reload } = usePipelineContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (pipelineId: string, stageNumber: number, memberId: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.STAGE_MEMBER(pipelineId, stageNumber, memberId))
      reload()
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to remove stage member'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { mutate, loading, error }
}

export { useApproveGate, useRenderStage, useSideTasks, useCreatePipeline, useUpdatePipeline, useDeletePipeline, useStartPipelineRun, useCancelPipelineRun, useAddStageMember, useRemoveStageMember }
