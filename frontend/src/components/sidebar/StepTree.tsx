import { useMemo } from 'react'
import Box from '@mui/material/Box'
import { useStore, workflowStore, workflowExecutionStore, sidebarStore } from '@/stores'
import { EmptyState } from '@/components/primitives'
import { StepTreeRow } from './StepTreeRow'
import { buildStepTree } from './buildStepTree'

// ── Output Parsing ──────────────────────────────────────────────────────────

/**
 * For workforce steps the raw output is `{"agents":{"name":"output",...}}`.
 * Extract the last agent's output as the step's display content.
 * Returns the original string unchanged for non-workforce / unparseable output.
 */
const extractFinalOutput = (raw: string): string => {
  try {
    const parsed: unknown = JSON.parse(raw)
    if (typeof parsed !== 'object' || parsed === null) return raw
    const agents = (parsed as Record<string, unknown>).agents
    if (typeof agents !== 'object' || agents === null) return raw

    const entries = Object.entries(agents as Record<string, unknown>)
    if (entries.length === 0) return raw

    const lastValue = entries[entries.length - 1]![1]
    if (typeof lastValue === 'string') return lastValue
    return JSON.stringify(lastValue, null, 2)
  } catch {
    return raw
  }
}

// ── Component ───────────────────────────────────────────────────────────────

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

        if (entry.kind === 'agent') {
          return null
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
            output={stepState?.output ? extractFinalOutput(stepState.output) : null}
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
