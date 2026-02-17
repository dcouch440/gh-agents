import { memo } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore, stepStreamStore } from '@/stores'
import { StreamView, ToolActivityFeed } from '../../execution'

type AgentStreamTabProps = {
  rosterAgentId: string
}

function AgentStreamTabComponent({ rosterAgentId }: AgentStreamTabProps) {
  const source = useStore(stepStreamStore.store, stepStreamStore.selectSource(rosterAgentId))

  const status = source?.status ?? 'idle'
  const isActive = status === 'running' || status === 'completed' || status === 'failed'

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
