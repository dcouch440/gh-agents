import { useState, useCallback } from 'react'
import { api } from '../api'
import type { Config, UpdateConfigRequest } from '../types'

const useUpdateConfig = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (body: UpdateConfigRequest): Promise<Config> => {
    setLoading(true)
    setError(null)
    try {
      return await api.patch<Config>('/config', body)
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to update config'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

export { useUpdateConfig }
