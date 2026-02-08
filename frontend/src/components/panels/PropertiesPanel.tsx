import { useMemo } from 'react'
import TuneOutlined from '@mui/icons-material/TuneOutlined'
import { EmptyState } from '@/components/primitives'
import { useStore, canvasStore, workflowStore } from '@/stores'
import { StepProperties } from './StepProperties'
import { EdgeProperties } from './EdgeProperties'

function PropertiesPanel() {
  const selectedStepIds = useStore(canvasStore.store, canvasStore.selectSelectedStepIds)
  const selectedEdgeIds = useStore(canvasStore.store, canvasStore.selectSelectedEdgeIds)
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const edges = useStore(workflowStore.store, workflowStore.selectEdges)

  const selectedStep = useMemo(() => {
    const firstId = selectedStepIds.values().next().value
    if (!firstId) return null
    return steps.find((s) => s.id === firstId) ?? null
  }, [selectedStepIds, steps])

  const selectedEdge = useMemo(() => {
    if (selectedStep) return null
    const firstId = selectedEdgeIds.values().next().value
    if (!firstId) return null
    return edges.find((e) => e.id === firstId) ?? null
  }, [selectedEdgeIds, edges, selectedStep])

  if (selectedStep) {
    return <StepProperties step={selectedStep} edges={edges} steps={steps} />
  }

  if (selectedEdge) {
    return <EdgeProperties edge={selectedEdge} steps={steps} />
  }

  return (
    <EmptyState
      icon={<TuneOutlined fontSize="large" />}
      message="Select a node to view properties"
    />
  )
}

export { PropertiesPanel }
