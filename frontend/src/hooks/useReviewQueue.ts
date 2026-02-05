import { useContext } from 'react'
import { ReviewQueueContext } from '@/contexts/ReviewQueueContext'
import type { ReviewQueueContextValue } from '@/contexts/ReviewQueueContext'

const useReviewQueue = (): ReviewQueueContextValue => {
  const ctx = useContext(ReviewQueueContext)
  if (!ctx) {
    throw new Error('useReviewQueue must be used within a ReviewQueueProvider')
  }
  return ctx
}

export { useReviewQueue }
