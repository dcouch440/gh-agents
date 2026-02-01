import { useState, useCallback } from 'react'
import { api } from '@/api'
import type { Task, CreateTaskRequest } from '@/types'

const useCreateTask = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: CreateTaskRequest): Promise<Task> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<Task>('/tasks', body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to create task'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

export { useCreateTask }
