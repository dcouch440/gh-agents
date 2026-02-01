import { useState, useEffect, useCallback } from 'react'
import type { FeedItem } from '@/types/feed'
import { USE_MOCK_DATA } from '@/constants'
import { mock } from '@/mock'

const useFeed = () => {
  const [items, setItems] = useState<FeedItem[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await mock.getFeed()
      setItems(data)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load feed')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (!USE_MOCK_DATA) {
      setLoading(false)
      return
    }

    let cancelled = false
    const run = async () => {
      await load()
      if (cancelled) return
    }
    run()
    return () => { cancelled = true }
  }, [load])

  return { items, loading, error, reload: load }
}

export { useFeed }
