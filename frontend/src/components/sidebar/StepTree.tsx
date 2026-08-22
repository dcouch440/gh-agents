import { useMemo } from 'react'
import Box from '@mui/material/Box'
import { useStore, workflowStore, workflowExecutionStore, workflowLiveStore, sidebarStore } from '@/stores'
import { dispatchStore } from '@/stores/dispatchStore'
import { Collections } from '@/utils/collections'
import { stepStreamStore } from '@/stores/stepStreamStore'
import { EmptyState } from '@/components/primitives'
import { StepTreeRow } from './StepTreeRow'
import { AgentTreeRow } from './AgentTreeRow'
import { buildStepTree, toAgentSlug } from './buildStepTree'
import { resolveNodeStatus } from './resolveNodeStatus'
import type { SourceStreamStatus } from '@/stores/stepStreamStore'

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
  const designStatusByStep = useStore(stepStreamStore.store, stepStreamStore.selectDesignStatusByStep)
  const sources = useStore(stepStreamStore.store, stepStreamStore.selectAllSources)
  const expandedAgentKeys = useStore(sidebarStore.store, sidebarStore.selectExpandedAgentKeys)
  const baselineByStep = useStore(workflowLiveStore.store, workflowLiveStore.selectBaselineByStep)
  const dispatches = useStore(workflowLiveStore.store, workflowLiveStore.selectDispatches)
  const dispatchEntries = useStore(dispatchStore.store, dispatchStore.selectByStep)

  const dispatchByStep = useMemo(
    () => Collections.keyBy(dispatches, (d) => d.stepId),
    [dispatches],
  )

  const entries = useMemo(() => {
    const result = buildStepTree(steps, edges, rosterByStep)
    if (import.meta.env.DEV) {
      console.warn('[StepTree] steps=%d edges=%d entries=%d', steps.length, edges.length, result.length)
    }
    return result
  }, [steps, edges, rosterByStep])

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
          const agentSlug = toAgentSlug(entry.agentName)
          const agentDesignState = designStatusByStep[entry.stepId]
          const isDesigned = agentDesignState?.designedAgentSlugs.has(agentSlug) === true

          const agentDesignStatus: SourceStreamStatus | null =
            isDesigned ? 'completed'
            : agentDesignState?.status === 'running' ? 'idle'
            : agentDesignState?.status === 'completed' ? 'completed'
            : null

          const agentSource = sources[entry.agentId]
          const executionStatus = agentSource?.status ?? null
          const agentOutput = agentSource?.streamBuffer ?? null
          const agentKey = `${entry.stepId}:${entry.agentId}`

          return (
            <AgentTreeRow
              key={agentKey}
              agentName={entry.agentName}
              agentId={entry.agentId}
              stepId={entry.stepId}
              gutter={entry.gutter}
              output={agentOutput}
              isExpanded={expandedAgentKeys[agentKey] === true}
              onToggle={() => { sidebarStore.toggleAgent(agentKey) }}
              designStatus={agentDesignStatus}
              executionStatus={executionStatus}
            />
          )
        }

        const stepState = stepStates[entry.step.id]
        const dispatch = dispatchByStep.get(entry.step.id) ?? null
        const resolved = resolveNodeStatus({
          baseline: baselineByStep[entry.step.id] ?? null,
          runState: stepState,
          dispatch,
        })

        return (
          <StepTreeRow
            key={entry.step.id}
            stepId={entry.step.id}
            name={entry.step.name ?? entry.step.description}
            executionMode={entry.step.execution_mode}
            gutter={entry.gutter}
            status={resolved.status}
            output={stepState?.output ? extractFinalOutput(stepState.output) : null}
            error={stepState?.error ?? null}
            isExpanded={expandedStepIds[entry.step.id] === true}
            isOutputExpanded={outputExpandedStepIds[entry.step.id] === true}
            onToggle={() => { sidebarStore.toggleStep(entry.step.id) }}
            onToggleOutputExpand={() => { sidebarStore.toggleOutputExpand(entry.step.id) }}
            designStatus={resolved.designStatus}
            designProgress={dispatchEntries[entry.step.id]?.message ?? null}
            pinned={resolved.pinned || entry.step.pinned}
            onTogglePin={() => { void workflowStore.togglePin(entry.step.id, !entry.step.pinned) }}
          />
        )
      })}
    </Box>
  )
}

export { StepTree }
