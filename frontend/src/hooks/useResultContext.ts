import { useContext } from 'react'
import { ResultContext } from '@/contexts/ResultContext'

const useResultContext = () => {
  const ctx = useContext(ResultContext)
  if (!ctx) throw new Error('useResultContext must be used within ResultProvider')
  return ctx
}

export { useResultContext }
