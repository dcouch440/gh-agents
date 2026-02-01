import { useContext } from 'react'
import { AgentContext } from '../contexts/AgentContext'

const useAgentContext = () => {
  const ctx = useContext(AgentContext)
  if (!ctx) throw new Error('useAgentContext must be used within AgentProvider')
  return ctx
}

export { useAgentContext }
