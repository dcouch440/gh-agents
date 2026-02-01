import { useContext } from 'react'
import { FeedContext } from '@/contexts/FeedContext'

const useFeedContext = () => {
  const ctx = useContext(FeedContext)
  if (!ctx) throw new Error('useFeedContext must be used within FeedProvider')
  return ctx
}

export { useFeedContext }
