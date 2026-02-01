import { useState, useEffect, useCallback } from 'react'
import type { Tool } from '@/types/tool'
import { API, USE_MOCK_DATA } from '@/constants'
import { mock } from '@/mock'
import { api } from '@/api'

const useTools = () => {
  const [tools, setTools] = useState<Tool[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = USE_MOCK_DATA
        ? await mock.getTools()
        : await api.get<Tool[]>(API.TOOLS)
      setTools(data)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load tools')
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

  return { tools, loading, error, reload: load }
}

export { useTools }
