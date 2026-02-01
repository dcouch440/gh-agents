import { useState, useEffect, useCallback } from 'react'
import type { Task, TaskStatus } from '@/types/task'
import { API, USE_MOCK_DATA } from '@/constants'
import { mock } from '@/mock'
import { api } from '@/api'

const useTasks = (statusFilter?: TaskStatus) => {
  const [tasks, setTasks] = useState<Task[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = USE_MOCK_DATA
        ? await mock.getTasks()
        : await api.get<Task[]>(statusFilter ? `${API.TASKS}?status=${statusFilter}` : API.TASKS)
      setTasks(statusFilter && USE_MOCK_DATA ? data.filter((t) => t.status === statusFilter) : data)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load tasks')
    } finally {
      setLoading(false)
    }
  }, [statusFilter])

  useEffect(() => {
    let cancelled = false
    const run = async () => {
      await load()
      if (cancelled) return
    }
    void run()
    return () => { cancelled = true }
  }, [load])

  return { tasks, loading, error, reload: load }
}

const useTask = (id: string | null) => {
  const [task, setTask] = useState<Task | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!id) {
      setTask(null)
      setLoading(false)
      return
    }

    let cancelled = false
    const load = async () => {
      setLoading(true)
      setError(null)
      try {
        const data = USE_MOCK_DATA
          ? await mock.getTask(id)
          : await api.get<Task>(API.TASK(id))
        if (!cancelled) setTask(data)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load task')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void load()
    return () => { cancelled = true }
  }, [id])

  return { task, loading, error }
}

export { useTasks, useTask }
