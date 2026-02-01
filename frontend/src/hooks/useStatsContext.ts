import { useContext } from 'react'
import { StatsContext } from '@/contexts/StatsContext'

const useStatsContext = () => {
  const ctx = useContext(StatsContext)
  if (!ctx) throw new Error('useStatsContext must be used within StatsProvider')
  return ctx
}

export { useStatsContext }
