import { useState, useEffect, useCallback } from 'react'
import type { ToolRouter } from '@/types'
import { api } from '@/api'

const useToolRouter = (id: string | null) => {
  const [router, setRouter] = useState<ToolRouter | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    if (!id) {
      setRouter(null)
      setLoading(false)
      return
    }
    setLoading(true)
    setError(null)
    try {
      const data = await api.toolRouters.get(id)
      setRouter(data)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load tool router')
    } finally {
      setLoading(false)
    }
  }, [id])

  useEffect(() => {
    if (!id) {
      setRouter(null)
      setLoading(false)
      return
    }

    let cancelled = false
    const run = async () => {
      setLoading(true)
      setError(null)
      try {
        const data = await api.toolRouters.get(id)
        if (!cancelled) setRouter(data)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Failed to load tool router')
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void run()
    return () => {
      cancelled = true
    }
  }, [id])

  return { router, loading, error, reload: load }
}

export { useToolRouter }
