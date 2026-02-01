import { useContext } from 'react'
import { PipelineContext } from '../contexts/PipelineContext'

const usePipelineContext = () => {
  const ctx = useContext(PipelineContext)
  if (!ctx) throw new Error('usePipelineContext must be used within PipelineProvider')
  return ctx
}

export { usePipelineContext }
