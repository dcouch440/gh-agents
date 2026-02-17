import { memo } from 'react'
import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { useStore, stepStreamStore } from '@/stores'
import { StreamView } from '../execution'
import { ToolActivityFeed } from '../execution'
import { BadgeList } from '../StepNode/BadgeList'
import { SectionLabel } from '../StepNode/SectionLabel'

type AgentNodeContentProps = {
  rosterAgentId: string
  roleDescription: string
  capabilities: string[]
}

function AgentNodeContentComponent({ rosterAgentId, roleDescription, capabilities }: AgentNodeContentProps) {
  const source = useStore(stepStreamStore.store, stepStreamStore.selectSource(rosterAgentId))

  const status = source?.status ?? 'idle'
  const isActive = status === 'running' || status === 'completed' || status === 'failed'

  if (!isActive) {
    return (
      <Box sx={{ px: 1.5, py: 1, display: 'flex', flexDirection: 'column', gap: 0.75 }}>
        {roleDescription !== '' && (
          <Box>
            <SectionLabel label="Role" />
            <Typography sx={{ fontSize: 10, color: 'text.secondary', lineHeight: 1.3 }}>
              {roleDescription}
            </Typography>
          </Box>
        )}
        {capabilities.length > 0 && (
          <Box>
            <SectionLabel label="Capabilities" />
            <BadgeList items={capabilities} />
          </Box>
        )}
      </Box>
    )
  }

  return (
    <Box
      className="nowheel nodrag nopan"
      sx={{
        flex: 1,
        minHeight: 0,
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
      }}
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

const AgentNodeContent = memo(AgentNodeContentComponent)

export { AgentNodeContent }
