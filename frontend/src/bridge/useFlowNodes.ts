// ============================================================================
// bridge/useFlowNodes — Store → React Flow nodes (memoized)
// ============================================================================

import { useMemo } from 'react'
import { useStore } from '@/stores/lib'
import { workflowStore } from '@/stores/workflowStore'
import { canvasStore } from '@/stores/canvasStore'
import { workflowExecutionStore } from '@/stores/workflowExecutionStore'
import type { WorkflowExecutionState, StepExecutionState } from '@/stores/workflowExecutionStore'
import { stepToNode } from './transforms'
import type { StepNode } from './types'

const selectStepStates = (s: WorkflowExecutionState): Record<string, StepExecutionState> =>
  s.stepStates

const useFlowNodes = (): StepNode[] => {
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const selectedIds = useStore(canvasStore.store, canvasStore.selectSelectedStepIds)
  const hoveredId = useStore(canvasStore.store, canvasStore.selectHoveredStepId)
  const stepStates = useStore(workflowExecutionStore.store, selectStepStates)

  return useMemo(
    () =>
      steps.map((step) =>
        stepToNode(
          step,
          stepStates[step.id] ?? null,
          selectedIds.has(step.id),
          hoveredId === step.id,
        ),
      ),
    [steps, stepStates, selectedIds, hoveredId],
  )
}

export { useFlowNodes }
