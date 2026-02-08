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

  const firstStepId = useMemo(
    () => selectedStepIds.values().next().value ?? null,
    [selectedStepIds],
  )
  const selectedStep = useStore(workflowStore.store, workflowStore.selectStepById(firstStepId))

  const firstEdgeId = useMemo(() => {
    if (selectedStep) return null
    return selectedEdgeIds.values().next().value ?? null
  }, [selectedEdgeIds, selectedStep])
  const selectedEdge = useStore(workflowStore.store, workflowStore.selectEdgeById(firstEdgeId))

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
