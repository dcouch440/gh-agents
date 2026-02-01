import { useState, useEffect, useCallback } from 'react'
import type { Config } from '@/types/config'
import { API, USE_MOCK_DATA } from '@/constants'
import { mock } from '@/mock'
import { api } from '@/api'

const useConfig = () => {
  const [config, setConfig] = useState<Config | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = USE_MOCK_DATA
        ? await mock.getConfig()
        : await api.get<Config>(API.CONFIG)
      setConfig(data)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load config')
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

  const updateConfig = useCallback(async (patch: Partial<Config>) => {
    if (USE_MOCK_DATA) {
      setConfig((prev) => prev ? { ...prev, ...patch } : null)
      return
    }
    const updated = await api.patch<Config>(API.CONFIG, patch)
    setConfig(updated)
  }, [])

  return { config, loading, error, reload: load, updateConfig }
}

export { useConfig }
