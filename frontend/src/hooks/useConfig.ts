import { useState, useEffect, useCallback } from 'react'
import type { Config, UpdateConfigRequest } from '@/types/config'
import { api } from '@/api'

const useConfig = () => {
  const [config, setConfig] = useState<Config | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.config.get()
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

  const updateConfig = useCallback(async (patch: UpdateConfigRequest) => {
    const updated = await api.config.update(patch)
    setConfig(updated)
  }, [])

  return { config, loading, error, reload: load, updateConfig }
}

export { useConfig }
