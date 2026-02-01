import { useState, useEffect, useCallback } from 'react'
import type { Pipeline, PipelineRun, StageExecution } from '@/types/pipeline'
import { API, USE_MOCK_DATA } from '@/constants'
import { mock } from '@/mock'
import { api } from '@/api'

const usePipelines = () => {
  const [pipelines, setPipelines] = useState<Pipeline[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = USE_MOCK_DATA
        ? await mock.getPipelines()
        : await api.get<Pipeline[]>(API.PIPELINES)
      setPipelines(data)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load pipelines')
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

  return { pipelines, loading, error, reload: load }
}

const usePipelineRuns = (pipelineId?: string) => {
  const [runs, setRuns] = useState<PipelineRun[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = USE_MOCK_DATA
        ? await mock.getPipelineRuns(pipelineId)
        : await api.get<PipelineRun[]>(pipelineId ? `${API.PIPELINE_RUNS}?pipeline_id=${pipelineId}` : API.PIPELINE_RUNS)
      setRuns(data)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load pipeline runs')
    } finally {
      setLoading(false)
    }
  }, [pipelineId])

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      await load()
      if (cancelled) return
    }
    run()
    return () => { cancelled = true }
  }, [load])

  return { runs, loading, error, reload: load }
}

const usePipelineRun = (id: string | null) => {
  const [run, setRun] = useState<PipelineRun | null>(null)
  const [executions, setExecutions] = useState<StageExecution[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!id) {
      setRun(null)
      setExecutions([])
      setLoading(false)
      return
    }

    let cancelled = false
    const load = async () => {
      setLoading(true)
      setError(null)
      try {
        if (USE_MOCK_DATA) {
          const [r, e] = await Promise.all([
            mock.getPipelineRun(id),
            mock.getStageExecutions(id),
          ])
          if (!cancelled) {
            setRun(r)
            setExecutions(e)
          }
        } else {
          const data = await api.get<PipelineRun & { stage_executions?: StageExecution[] }>(API.PIPELINE_RUN(id))
          if (!cancelled) {
            const { stage_executions, ...rest } = data
            setRun(rest)
            setExecutions(stage_executions ?? [])
          }
        }
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load pipeline run')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    load()
    return () => { cancelled = true }
  }, [id])

  return { run, executions, loading, error }
}

export { usePipelines, usePipelineRuns, usePipelineRun }
