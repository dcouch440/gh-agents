import { useState, useCallback } from 'react'
import { api } from '@/api'
import { API } from '@/constants'

type ContextResponseRequest = {
  agent_id: string
  response: string
}

const useContextResponse = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: ContextResponseRequest): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.post(API.CONTEXT_RESPONSE, body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to submit context response'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

export { useContextResponse }
export type { ContextResponseRequest }
