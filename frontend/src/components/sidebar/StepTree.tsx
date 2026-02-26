import { useMemo } from 'react'
import Box from '@mui/material/Box'
import { useStore, workflowStore, sidebarStore } from '@/stores'
import { EmptyState } from '@/components/primitives'
import { StepTreeRow } from './StepTreeRow'
import { buildStepTree } from './buildStepTree'

function StepTree() {
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const edges = useStore(workflowStore.store, workflowStore.selectEdges)
  const rosterByStep = useStore(workflowStore.store, workflowStore.selectRosterByStep)
  const selectedStepId = useStore(sidebarStore.store, sidebarStore.selectSelectedStepId)

  const entries = useMemo(() => buildStepTree(steps, edges, rosterByStep), [steps, edges, rosterByStep])

  if (entries.length === 0) {
    return (
      <EmptyState message="Draw boxes on the canvas and submit to see your workflow here." />
    )
  }

  return (
    <Box role="tree" sx={{ py: 0.5 }}>
      {entries.map((entry, i) => {
        if (entry.kind === 'gap') {
          return <Box key={`gap-${String(i)}`} sx={{ height: 8 }} />
        }

        return (
          <StepTreeRow
            key={entry.step.id}
            name={entry.step.name ?? entry.step.description}
            executionMode={entry.step.execution_mode}
            gutter={entry.gutter}
            isSelected={entry.step.id === selectedStepId}
            onClick={() => {
              if (selectedStepId === entry.step.id) {
                sidebarStore.clearSelection()
              } else {
                sidebarStore.selectStep(entry.step.id)
              }
            }}
          />
        )
      })}
    </Box>
  )
}

export { StepTree }
