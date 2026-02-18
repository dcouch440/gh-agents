import { memo, useMemo } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore, stepStreamStore, workflowExecutionStore } from '@/stores'
import { StreamView, ToolActivityFeed } from '../../execution'

type AgentStreamTabProps = {
  rosterAgentId: string
  protocolStepId: string | null
  agentName: string
}

/**
 * Extract a single agent's output from a workforce step's persisted output JSON.
 * Workforce output shape: { agents: { "writer": "text...", "critic": "text..." }, ... }
 */
const extractAgentOutput = (raw: string, agentName: string): string | null => {
  try {
    const parsed: unknown = JSON.parse(raw)
    if (parsed === null || typeof parsed !== 'object') return null

    const obj = parsed as Record<string, unknown>
    if (typeof obj.agents !== 'object' || obj.agents === null) return null

    const agents = obj.agents as Record<string, unknown>
    const normalizedName = agentName.toLowerCase()
    const content = agents[normalizedName]
    return typeof content === 'string' ? content : null
  } catch {
    return null
  }
}

function AgentStreamTabComponent({ rosterAgentId, protocolStepId, agentName }: AgentStreamTabProps) {
  const source = useStore(stepStreamStore.store, stepStreamStore.selectSource(rosterAgentId))

  const parentStepExec = useStore(
    workflowExecutionStore.store,
    protocolStepId ? workflowExecutionStore.selectStepState(protocolStepId) : () => null,
  )

  const status = source?.status ?? 'idle'
  const isActive = status === 'running' || status === 'completed' || status === 'failed'

  const persistedContent = useMemo(() => {
    if (isActive) return null
    const parentOutput = parentStepExec?.output ?? null
    if (parentOutput === null) return null
    return extractAgentOutput(parentOutput, agentName)
  }, [isActive, parentStepExec?.output, agentName])

  if (!isActive && persistedContent !== null) {
    return (
      <Box
        className="nowheel nodrag nopan"
        sx={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}
      >
        <Box sx={{ flex: 1, minHeight: 0, px: 1, py: 0.5 }}>
          <StreamView content={persistedContent} status="completed" />
        </Box>
      </Box>
    )
  }

  if (!isActive) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
        <Typography variant="body2" color="text.secondary">
          No live data yet. Run the workflow to see streaming output.
        </Typography>
      </Box>
    )
  }

  return (
    <Box
      className="nowheel nodrag nopan"
      sx={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}
    >
      <Box sx={{ flex: 1, minHeight: 0, px: 1, py: 0.5 }}>
        <StreamView
          content={source?.streamBuffer ?? ''}
          status={status === 'completed' || status === 'failed' ? status : 'running'}
          error={source?.error}
        />
      </Box>

      {source !== null && source.toolUses.length > 0 && (
        <Box sx={{ px: 1.5, py: 0.5, borderTop: 1, borderColor: 'divider', flexShrink: 0 }}>
          <ToolActivityFeed tools={source.toolUses} compact />
        </Box>
      )}
    </Box>
  )
}

const AgentStreamTab = memo(AgentStreamTabComponent)

export { AgentStreamTab }
export type { AgentStreamTabProps }
