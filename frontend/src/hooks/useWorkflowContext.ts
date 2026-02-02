import { useContext } from 'react'
import { WorkflowContext } from '@/contexts/WorkflowContext'

const useWorkflowContext = () => {
  const ctx = useContext(WorkflowContext)
  if (!ctx) throw new Error('useWorkflowContext must be used within WorkflowProvider')
  return ctx
}

export { useWorkflowContext }
