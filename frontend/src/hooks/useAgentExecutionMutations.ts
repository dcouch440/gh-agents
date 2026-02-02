import { useState, useCallback } from 'react'
import { api } from '@/api'
import { API } from '@/constants'

const useCancelAgentExecution = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (executionId: string): Promise<{ status: string }> => {
    setLoading(true)
    setError(null)
    try {
      return await api.post<{ status: string }>(API.AGENT_EXECUTION_CANCEL(executionId))
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to cancel agent execution'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

export { useCancelAgentExecution }
