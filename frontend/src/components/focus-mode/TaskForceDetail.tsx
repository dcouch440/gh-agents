import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import Chip from '@mui/material/Chip'
import { useStore, workflowStore } from '@/stores'
import { DetailShell } from './DetailShell'
import type { RosterAgent } from '@/types/workflow'

type TaskForceDetailProps = {
  stepId: string
  onClose: () => void
}

function TaskForceDetail({ stepId, onClose }: TaskForceDetailProps) {
  const step = useStore(workflowStore.store, workflowStore.selectStepById(stepId))
  const rosterByStep = useStore(workflowStore.store, workflowStore.selectRosterByStep)
  const roster = rosterByStep[stepId] ?? []

  const title = step?.name ?? 'Task Force'

  return (
    <DetailShell title={title} accentColor="#3b82f6" onClose={onClose}>
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {step?.description && (
          <Typography sx={{ fontSize: 13, color: 'text.secondary', lineHeight: 1.5 }}>
            {step.description}
          </Typography>
        )}
        <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
          <Chip label={`${roster.length} agent${roster.length !== 1 ? 's' : ''}`} size="small" variant="outlined" />
        </Box>
        {roster.length > 0 ? (
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
            {roster.map((agent: RosterAgent) => (
              <Box
                key={agent.id}
                sx={{
                  p: 1.5,
                  borderRadius: '8px',
                  border: 1,
                  borderColor: 'divider',
                  backgroundColor: (theme) => theme.palette.custom.hoverOverlay,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 0.5,
                }}
              >
                <Typography sx={{ fontSize: 12, fontWeight: 600, color: 'text.primary' }}>
                  {agent.name}
                </Typography>
                {agent.role_description && (
                  <Typography sx={{ fontSize: 11, color: 'text.secondary', lineHeight: 1.4 }}>
                    {agent.role_description}
                  </Typography>
                )}
                {agent.capabilities.length > 0 && (
                  <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5, mt: 0.5 }}>
                    {agent.capabilities.map((cap, i) => (
                      <Chip key={i} label={cap} size="small" variant="outlined" sx={{ fontSize: 10 }} />
                    ))}
                  </Box>
                )}
              </Box>
            ))}
          </Box>
        ) : (
          <Typography sx={{ fontSize: 12, color: 'text.disabled', fontStyle: 'italic' }}>
            No agents in roster yet
          </Typography>
        )}
      </Box>
    </DetailShell>
  )
}

export { TaskForceDetail }
export type { TaskForceDetailProps }
