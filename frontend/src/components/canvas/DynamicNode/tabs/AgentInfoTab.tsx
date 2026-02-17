import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import { BadgeList } from '../../StepNode/BadgeList'
import { SectionLabel } from '../../StepNode/SectionLabel'

type AgentInfoTabProps = {
  roleDescription: string
  capabilities: string[]
}

function AgentInfoTab({ roleDescription, capabilities }: AgentInfoTabProps) {
  const hasRole = roleDescription !== ''
  const hasCaps = capabilities.length > 0

  if (!hasRole && !hasCaps) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', p: 2 }}>
        <Typography variant="body2" color="text.secondary">
          No agent details configured.
        </Typography>
      </Box>
    )
  }

  return (
    <Box sx={{ px: 1.5, py: 1, display: 'flex', flexDirection: 'column', gap: 0.75 }}>
      {hasRole && (
        <Box>
          <SectionLabel label="Role" />
          <Typography sx={{ fontSize: 10, color: 'text.secondary', lineHeight: 1.3 }}>
            {roleDescription}
          </Typography>
        </Box>
      )}
      {hasCaps && (
        <Box>
          <SectionLabel label="Capabilities" />
          <BadgeList items={capabilities} />
        </Box>
      )}
    </Box>
  )
}

export { AgentInfoTab }
export type { AgentInfoTabProps }
