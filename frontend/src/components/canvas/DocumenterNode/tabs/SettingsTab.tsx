import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'

type SettingsTabProps = {
  modelId: string | null
  agentName: string | null
}

function SettingsTab({ modelId, agentName }: SettingsTabProps) {
  return (
    <Box sx={{ p: 1.5, display: 'flex', flexDirection: 'column', gap: 1.5 }}>
      <Box>
        <Typography
          sx={{
            fontSize: 8,
            fontWeight: 600,
            textTransform: 'uppercase',
            color: 'text.disabled',
            letterSpacing: '0.06em',
            lineHeight: 1,
            mb: 0.5,
          }}
        >
          Agent
        </Typography>
        <Typography sx={{ fontSize: 12, color: agentName !== null ? 'text.primary' : 'text.disabled' }}>
          {agentName ?? 'Not configured'}
        </Typography>
      </Box>
      <Box>
        <Typography
          sx={{
            fontSize: 8,
            fontWeight: 600,
            textTransform: 'uppercase',
            color: 'text.disabled',
            letterSpacing: '0.06em',
            lineHeight: 1,
            mb: 0.5,
          }}
        >
          Model
        </Typography>
        <Typography sx={{ fontSize: 12, color: modelId !== null ? 'text.primary' : 'text.disabled' }}>
          {modelId ?? 'Not configured'}
        </Typography>
      </Box>
    </Box>
  )
}

export { SettingsTab }
export type { SettingsTabProps }
