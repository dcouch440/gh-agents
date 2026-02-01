import { useState, useEffect, useCallback } from 'react'
import type { UsageSummary } from '@/types/stats'
import { USE_MOCK_DATA, STATS_POLL_INTERVAL_MS } from '@/constants'
import { mock } from '@/mock'
import { api } from '@/api'

const useStats = () => {
  const [stats, setStats] = useState<UsageSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      const data = USE_MOCK_DATA
        ? await mock.getStats()
        : await api.get<UsageSummary[]>('/stats')
      setStats(data)
      setError(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load stats')
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

    if (!USE_MOCK_DATA) {
      const interval = setInterval(() => {
        if (!cancelled) load()
      }, STATS_POLL_INTERVAL_MS)
      return () => { cancelled = true; clearInterval(interval) }
    }

    return () => { cancelled = true }
  }, [load])

  return { stats, loading, error, reload: load }
}

export { useStats }
