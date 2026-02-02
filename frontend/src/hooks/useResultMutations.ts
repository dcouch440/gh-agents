import { useState, useCallback } from 'react'
import { api } from '@/api'
import { API } from '@/constants'
import { useResultContext } from '@/hooks/useResultContext'

const useResultMutations = () => {
  const { reload } = useResultContext()
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const remove = useCallback(async (id: string): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del(API.RESULT(id))
      reload()
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to delete result'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [reload])

  return { remove, loading, error }
}

export { useResultMutations }
