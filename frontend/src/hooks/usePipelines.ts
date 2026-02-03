import { useState, useEffect, useCallback } from 'react'
import type { Pipeline, PipelineRun, StageExecution } from '@/types/pipeline'
import { api } from '@/api'

const usePipelines = () => {
  const [pipelines, setPipelines] = useState<Pipeline[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.pipelines.list()
      setPipelines(data.items)
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
    void run()
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
      const data = pipelineId
        ? await api.get<PipelineRun[]>(`${'/pipeline-runs'}?pipeline_id=${pipelineId}`)
        : await api.pipelineRuns.list()
      const items = Array.isArray(data) ? data : data.items
      setRuns(items)
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
    void run()
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
        const data = await api.pipelineRuns.get(id) as PipelineRun & { stage_executions?: StageExecution[] }
        if (!cancelled) {
          const { stage_executions, ...rest } = data
          setRun(rest)
          setExecutions(stage_executions ?? [])
        }
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load pipeline run')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void load()
    return () => { cancelled = true }
  }, [id])

  return { run, executions, loading, error }
}

export { usePipelines, usePipelineRuns, usePipelineRun }
