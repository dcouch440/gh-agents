import Box from '@mui/material/Box'
import Typography from '@mui/material/Typography'
import SmartToyOutlined from '@mui/icons-material/SmartToyOutlined'
import { ProtocolBadge } from '../ProtocolBadge'
import { AGENT_NODE } from './constants'

type AgentNodeHeaderProps = {
  name: string
  roleDescription: string
  parentStepName: string
  accentColor?: string
}

function AgentNodeHeader({ name, roleDescription, parentStepName, accentColor = AGENT_NODE.ACCENT_COLOR }: AgentNodeHeaderProps) {
  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, px: 1.5, width: '100%' }}>
      <Box
        sx={{
          width: 28,
          height: 28,
          borderRadius: '6px',
          backgroundColor: `${accentColor}20`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        <SmartToyOutlined sx={{ fontSize: 18, color: accentColor }} />
      </Box>

      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography
          sx={{
            fontSize: 13,
            fontWeight: 600,
            color: 'text.primary',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {name}
        </Typography>
        <Typography
          sx={{
            fontSize: 10,
            color: 'text.disabled',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            lineHeight: 1.2,
          }}
        >
          {roleDescription || parentStepName}
        </Typography>
      </Box>

      <Box sx={{ mr: 0.5 }}>
        <ProtocolBadge color={accentColor} label="Agent" />
      </Box>
    </Box>
  )
}

export { AgentNodeHeader }
export type { AgentNodeHeaderProps }
