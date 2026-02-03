import { useState } from 'react'
import type { FeedItem } from '@/types/feed'

const useFeed = () => {
  const [items] = useState<FeedItem[]>([])
  const [loading] = useState(false)
  const [error] = useState<string | null>(null)

  return { items, loading, error }
}

export { useFeed }
