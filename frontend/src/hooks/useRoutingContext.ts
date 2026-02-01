import { useContext } from 'react'
import { RoutingContext } from '../contexts/RoutingContext'

const useRoutingContext = () => {
  const ctx = useContext(RoutingContext)
  if (!ctx) throw new Error('useRoutingContext must be used within RoutingProvider')
  return ctx
}

export { useRoutingContext }
