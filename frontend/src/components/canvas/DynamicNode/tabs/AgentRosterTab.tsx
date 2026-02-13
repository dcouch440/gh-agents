import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type AgentRosterTabProps = {
  stepId: string
}

function AgentRosterTab({ stepId: _stepId }: AgentRosterTabProps) {
  return (
    <Box sx={{ p: 1.5, height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
      <Typography sx={{ fontSize: 12, color: 'text.disabled', textAlign: 'center' }}>
        No agents configured yet. Use the chat to define your team.
      </Typography>
    </Box>
  )
}

export { AgentRosterTab }
export type { AgentRosterTabProps }
