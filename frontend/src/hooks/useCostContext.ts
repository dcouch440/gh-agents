import { useContext } from 'react'
import { CostContext } from '@/contexts/CostContext'

const useCostContext = () => {
  const ctx = useContext(CostContext)
  if (!ctx) throw new Error('useCostContext must be used within CostProvider')
  return ctx
}

export { useCostContext }
