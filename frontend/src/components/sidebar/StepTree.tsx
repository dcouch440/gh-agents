import { useMemo } from 'react'
import Box from '@mui/material/Box'
import { useStore, workflowStore, workflowExecutionStore, sidebarStore } from '@/stores'
import { EmptyState } from '@/components/primitives'
import { StepTreeRow } from './StepTreeRow'
import { buildStepTree } from './buildStepTree'

function StepTree() {
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const edges = useStore(workflowStore.store, workflowStore.selectEdges)
  const rosterByStep = useStore(workflowStore.store, workflowStore.selectRosterByStep)
  const stepStates = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStepStates)
  const expandedStepIds = useStore(sidebarStore.store, sidebarStore.selectExpandedStepIds)
  const outputExpandedStepIds = useStore(sidebarStore.store, sidebarStore.selectOutputExpandedStepIds)

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

        const stepState = stepStates[entry.step.id]

        return (
          <StepTreeRow
            key={entry.step.id}
            stepId={entry.step.id}
            name={entry.step.name ?? entry.step.description}
            executionMode={entry.step.execution_mode}
            gutter={entry.gutter}
            status={stepState?.status}
            output={stepState?.output ?? null}
            error={stepState?.error ?? null}
            isExpanded={expandedStepIds[entry.step.id] === true}
            isOutputExpanded={outputExpandedStepIds[entry.step.id] === true}
            onToggle={() => { sidebarStore.toggleStep(entry.step.id) }}
            onToggleOutputExpand={() => { sidebarStore.toggleOutputExpand(entry.step.id) }}
          />
        )
      })}
    </Box>
  )
}

export { StepTree }
