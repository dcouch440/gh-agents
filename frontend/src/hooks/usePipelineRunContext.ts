import { useContext } from 'react'
import { PipelineRunContext } from '@/contexts/PipelineRunContext'

const usePipelineRunContext = () => {
  const ctx = useContext(PipelineRunContext)
  if (!ctx) throw new Error('usePipelineRunContext must be used within PipelineRunProvider')
  return ctx
}

export { usePipelineRunContext }
