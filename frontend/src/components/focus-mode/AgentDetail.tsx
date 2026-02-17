import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import { useStore, workflowStore } from '@/stores'
import { DetailShell } from './DetailShell'

type AgentDetailProps = {
  artifactId: string
  onClose: () => void
}

function AgentDetail({ artifactId, onClose }: AgentDetailProps) {
  const agent = useStore(workflowStore.store, workflowStore.selectRosterAgentById(artifactId))

  if (!agent) {
    return (
      <DetailShell title="Agent" accentColor="#3b82f6" onClose={onClose}>
        <Typography sx={{ color: 'text.disabled' }}>Agent not found</Typography>
      </DetailShell>
    )
  }

  return (
    <DetailShell title={agent.name} accentColor="#3b82f6" onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {agent.role_description && (
          <Box>
            <Typography sx={{ fontSize: 11, fontWeight: 700, color: 'text.secondary', mb: 0.5, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
              Role
            </Typography>
            <Typography sx={{ fontSize: 13, color: 'text.primary', lineHeight: 1.5 }}>
              {agent.role_description}
            </Typography>
          </Box>
        )}
        {agent.capabilities.length > 0 && (
          <Box>
            <Typography sx={{ fontSize: 11, fontWeight: 700, color: 'text.secondary', mb: 0.5, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
              Capabilities
            </Typography>
            <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
              {agent.capabilities.map((cap, i) => (
                <Chip key={i} label={cap} size="small" variant="outlined" />
              ))}
            </Box>
          </Box>
        )}
      </Box>
    </DetailShell>
  )
}

export { AgentDetail }
export type { AgentDetailProps }
