import { useMemo, useState } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Divider from '@mui/material/Divider'
import IconButton from '@mui/material/IconButton'
import ContentCopyRounded from '@mui/icons-material/ContentCopyRounded'
import CheckRounded from '@mui/icons-material/CheckRounded'
import { useStore } from '@/stores/lib'
import { agentTraceStore } from '@/stores/agentTraceStore'
import type { AgentTrace } from '@/stores/agentTraceStore'
import { activityStore } from '@/stores/activity'
import { workflowStore } from '@/stores/workflowStore'
import { Collections } from '@/utils/collections'
import { ActivityTimeline } from './ActivityTimeline'
import { AgentTraceCard } from './AgentTraceCard'
import { buildRunExport } from './exportDispatch'

/**
 * Run tab content — activity timeline + agent execution traces grouped by step.
 */
function RunTab() {
  const traces = useStore(agentTraceStore.store, agentTraceStore.selectTraces)
  const order = useStore(agentTraceStore.store, agentTraceStore.selectOrder)
  const activities = useStore(activityStore.store, activityStore.selectAll)
  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const [copied, setCopied] = useState(false)

  const stepNameMap = useMemo(
    () => Collections.toLookupMap(steps, (s) => s.id, (s) => s.name ?? s.id.slice(0, 8)),
    [steps],
  )

  // Group traces by step, preserving insertion order
  const tracesByStep = useMemo(() => {
    const groups = new Map<string, AgentTrace[]>()
    for (const id of order) {
      const trace = traces[id]
      if (trace === undefined) continue
      const existing = groups.get(trace.stepId)
      if (existing !== undefined) {
        existing.push(trace)
      } else {
        groups.set(trace.stepId, [trace])
      }
    }
    return groups
  }, [traces, order])

  const hasTraces = order.length > 0

  return (
    <>
      <Box sx={{ display: 'flex', justifyContent: 'flex-end', px: 1, py: 0.5 }}>
        <IconButton
          size="small"
          aria-label="Copy run JSON"
          onClick={() => {
            const data = buildRunExport()
            void navigator.clipboard.writeText(JSON.stringify(data, null, 2)).then(() => {
              setCopied(true)
              setTimeout(() => { setCopied(false) }, 1500)
            })
          }}
        >
          {copied
            ? <CheckRounded sx={{ fontSize: 14, color: 'success.main' }} />
            : <ContentCopyRounded sx={{ fontSize: 14, color: 'text.disabled' }} />}
        </IconButton>
      </Box>
      <ActivityTimeline activities={activities} />
      <Divider />

      {hasTraces ? (
        [...tracesByStep.entries()].map(([stepId, stepTraces]) => (
          <StepTraceGroup
            key={stepId}
            stepName={stepNameMap.get(stepId) ?? stepId.slice(0, 8)}
            traces={stepTraces}
          />
        ))
      ) : (
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', py: 4 }}>
          <Typography variant="body2" sx={{ color: 'text.disabled', fontStyle: 'italic' }}>
            No execution traces yet. Run the workflow to see agent activity.
          </Typography>
        </Box>
      )}
    </>
  )
}

// ── Step group ───────────────────────────────────────────────────────────────

type StepTraceGroupProps = {
  readonly stepName: string
  readonly traces: readonly AgentTrace[]
}

function StepTraceGroup({ stepName, traces }: StepTraceGroupProps) {
  return (
    <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
      <Typography
        sx={{
          px: 1.5,
          py: 0.5,
          fontSize: 11,
          fontWeight: 600,
          color: 'text.secondary',
          bgcolor: 'action.hover',
        }}
      >
        {stepName}
      </Typography>
      {traces.map((trace) => (
        <AgentTraceCard key={trace.agentExecutionId} trace={trace} />
      ))}
    </Box>
  )
}

export { RunTab }
